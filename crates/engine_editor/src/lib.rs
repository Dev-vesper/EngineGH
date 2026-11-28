//! Dear ImGui based editor overlay. Only compiled into builds that depend on this crate, so a
//! shipping build without it contains no ImGui code and no ImGui memory.
//!
//! Frame flow: `handle_event` for every window event, `update_stats` once per frame, then
//! `draw` from inside the renderer's overlay pass. UI edits take effect on the next frame.

mod panels;
mod state;
mod stats;

use std::fmt;
use std::sync::Arc;

use dear_imgui_rs::{ConfigFlags, Context};
use dear_imgui_wgpu::{FramebufferExtent, WgpuInitInfo, WgpuRenderer};
use dear_imgui_winit::{HiDpiMode, WinitPlatform};
use engine_render::wgpu;
use state::EditorState;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use engine_memory::MemorySampler;
pub use state::{EditorOutput, InspectorData, SceneOp, SelectRequest};
pub use stats::FrameStats;

#[derive(Debug)]
pub struct EditorError(String);

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "editor initialisation failed: {}", self.0)
    }
}

impl std::error::Error for EditorError {}

pub struct Editor {
    // Field order = drop order: the renderer must go before the context it is bound to.
    renderer: WgpuRenderer,
    platform: WinitPlatform,
    context: Context,
    window: Arc<Window>,
    visible: bool,
    /// The mouse is captured by the game view; UI pointer/key input is being ignored.
    input_blocked: bool,
    state: EditorState,
    stats: FrameStats,
    memory: MemorySampler,
    adapter_label: String,
}

impl Editor {
    pub fn new(
        window: Arc<Window>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        adapter: &wgpu::AdapterInfo,
    ) -> Result<Self, EditorError> {
        let err = |e: &dyn fmt::Display| EditorError(e.to_string());

        let mut context = Context::create();
        context.set_ini_filename(None::<&str>).map_err(|e| err(&e))?; // no imgui.ini on disk
        // The platform layer owns cursor visibility/grab; ImGui must not touch it.
        let flags = context.io().config_flags() | ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
        context.io_mut().set_config_flags(flags);

        let mut platform = WinitPlatform::new(&mut context).map_err(|e| err(&e))?;
        platform
            .attach_window(window.clone(), HiDpiMode::Default, &mut context)
            .map_err(|e| err(&e))?;

        let renderer = WgpuRenderer::new(WgpuInitInfo::new(device, queue, surface_format), &mut context)
            .map_err(|e| err(&e))?;

        Ok(Self {
            renderer,
            platform,
            context,
            window,
            visible: true,
            input_blocked: false,
            state: EditorState::default(),
            stats: FrameStats::new(),
            memory: MemorySampler::new(),
            adapter_label: format!("{} ({:?}, {:?})", adapter.name, adapter.device_type, adapter.backend),
        })
    }

    /// Feeds a window event to the UI. Returns `true` if the UI wants this event
    /// (see `engine_platform::App::window_event`). F1 toggles the editor.
    ///
    /// While `cursor_grabbed` the UI receives no pointer motion, wheel or press events, so a
    /// hidden/captured mouse can never click a widget. Releases still pass, so nothing sticks.
    pub fn handle_event(&mut self, id: WindowId, event: &WindowEvent, cursor_grabbed: bool) -> bool {
        if let WindowEvent::KeyboardInput { event: key, .. } = event {
            if key.state == ElementState::Pressed && !key.repeat && key.physical_key == PhysicalKey::Code(KeyCode::F1)
            {
                self.visible = !self.visible;
                return true;
            }
        }
        if !self.visible {
            return false;
        }
        if cursor_grabbed {
            if !self.input_blocked {
                self.input_blocked = true;
                // Move the UI pointer off-screen so hover and active states clear.
                self.context.io_mut().add_mouse_pos_event([f32::MIN, f32::MIN]);
            }
            if is_pointer_or_press(event) {
                return false;
            }
        } else {
            self.input_blocked = false;
        }
        let outcome = match self.platform.handle_window_event(&mut self.context, id, event) {
            Ok(o) => o,
            Err(e) => {
                log::error!("imgui platform event error: {e}");
                return false;
            }
        };
        let Some(capture) = outcome.capture else { return false };
        match event {
            WindowEvent::KeyboardInput { .. } | WindowEvent::Ime(_) => capture.keyboard || capture.text_input,
            WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } => capture.pointer,
            _ => false,
        }
    }

    /// Whether the overlay is currently shown (F1).
    #[inline]
    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn update_stats(&mut self, dt: f32) {
        self.stats.push(dt);
    }

    /// Builds the UI and records its draw commands into `pass` (the renderer's overlay pass).
    pub fn draw(
        &mut self,
        data: &mut InspectorData<'_>,
        out: &mut EditorOutput,
        pass: &mut wgpu::RenderPass<'_>,
        extent: (u32, u32),
    ) {
        if !self.visible || extent.0 == 0 || extent.1 == 0 {
            return;
        }
        if let Err(e) = self.platform.prepare_frame(&mut self.context, &self.window) {
            log::error!("imgui prepare_frame failed: {e}");
            return;
        }

        let memory = *self.memory.update();
        let ui = self.context.frame();
        panels::draw_all(
            ui,
            &mut self.state,
            &panels::PanelContext { stats: &self.stats, memory: &memory, adapter: &self.adapter_label },
            data,
            out,
        );
        if let Err(e) = self.platform.prepare_render(ui, &self.window) {
            log::error!("imgui prepare_render failed: {e}");
        }

        let consumer = match self.renderer.renderer_consumer() {
            Ok(c) => c,
            Err(e) => {
                log::error!("imgui renderer unavailable: {e}");
                return;
            }
        };
        let pending = self.context.render(consumer);
        if let Err(e) = self.renderer.render(pending, pass, FramebufferExtent::new(extent.0, extent.1)) {
            log::error!("imgui render failed: {e}");
        }
    }
}

/// Events that must not reach the UI while the game view owns the mouse.
fn is_pointer_or_press(event: &WindowEvent) -> bool {
    match event {
        WindowEvent::CursorMoved { .. } | WindowEvent::MouseWheel { .. } | WindowEvent::Ime(_) => true,
        WindowEvent::MouseInput { state, .. } => *state == ElementState::Pressed,
        WindowEvent::KeyboardInput { event, .. } => event.state == ElementState::Pressed,
        _ => false,
    }
}
