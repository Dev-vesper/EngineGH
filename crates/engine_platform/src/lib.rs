//! Window + event loop + input. The only crate that knows about `winit`.

mod input;

use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

pub use input::Input;
pub use winit::keyboard::KeyCode as Key;
pub use winit::dpi::PhysicalSize;
pub use winit::event::WindowEvent;
pub use winit::window::WindowId;
pub use winit::window::Window as PlatformWindow;

/// Upper bound on a single frame delta, so a stall (drag, breakpoint) does not teleport the camera.
const MAX_DT: f32 = 0.1;

pub struct WindowConfig {
    pub title: String,
    pub logical_size: (f64, f64),
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { title: "Engine".to_owned(), logical_size: (1280.0, 720.0) }
    }
}

/// Application callbacks driven by the platform loop.
pub trait App {
    /// Called once, after the OS window exists. The `Arc<Window>` implements
    /// `raw_window_handle::{HasWindowHandle, HasDisplayHandle}`.
    fn init(&mut self, window: Arc<Window>);
    fn resize(&mut self, size: PhysicalSize<u32>);
    fn update(&mut self, dt: f32, input: &Input);
    fn render(&mut self);

    /// Sees every window event first (e.g. a GUI layer). Return `true` if the event is captured
    /// by the UI: *press* events (key / mouse button) that are captured never reach `Input` and
    /// never start a mouse grab. Releases are always delivered so no key can stay stuck down.
    /// The return value is ignored while the cursor is grabbed.
    fn window_event(&mut self, _id: WindowId, _event: &WindowEvent) -> bool {
        false
    }

    /// Polled after every rendered frame; return `true` to close the window and end `run`.
    fn should_exit(&self) -> bool {
        false
    }
}

pub fn run<A: App>(config: WindowConfig, app: A) -> Result<(), winit::error::EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut runner = Runner {
        config,
        app,
        window: None,
        input: Input::default(),
        last_frame: Instant::now(),
    };
    event_loop.run_app(&mut runner)
}

struct Runner<A: App> {
    config: WindowConfig,
    app: A,
    window: Option<Arc<Window>>,
    input: Input,
    last_frame: Instant,
}

impl<A: App> Runner<A> {
    fn set_grab(&mut self, grab: bool) {
        let Some(window) = self.window.as_ref() else { return };
        if grab {
            // Locked is unsupported on some platforms (X11, Windows); Confined is the fallback.
            let ok = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .is_ok();
            window.set_cursor_visible(!ok);
            self.input.set_cursor_grabbed(ok);
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.input.set_cursor_grabbed(false);
        }
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.logical_size.0, self.config.logical_size.1));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("window creation failed: {e}");
                event_loop.exit();
                return;
            }
        };
        self.app.init(window.clone());
        self.window = Some(window);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let captured = self.app.window_event(id, &event) && !self.input.cursor_grabbed();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.app.resize(size),
            WindowEvent::Focused(false) => {
                self.input.clear_all();
                self.set_grab(false);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. }
                if !captured && !self.input.cursor_grabbed() =>
            {
                self.set_grab(true);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    if pressed && captured {
                        return;
                    }
                    if pressed && code == KeyCode::Escape {
                        if self.input.cursor_grabbed() {
                            self.set_grab(false);
                        } else {
                            event_loop.exit();
                        }
                    }
                    self.input.set_key(code, pressed);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = self.window.as_ref() else { return };
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32().min(MAX_DT);
                self.last_frame = now;

                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return; // minimised
                }
                self.app.update(dt, &self.input);
                self.app.render();
                self.input.end_frame();
                if self.app.should_exit() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.add_mouse_delta(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}
