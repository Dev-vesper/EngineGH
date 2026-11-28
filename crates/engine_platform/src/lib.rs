//! Window + event loop + input. The only crate that knows about `winit`.

mod input;

use std::sync::Arc;
use std::time::Instant;

use engine_time::FixedTimestep;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window};

pub use input::Input;
pub use winit::keyboard::KeyCode as Key;
pub use winit::dpi::PhysicalSize;
pub use winit::event::{ElementState, MouseButton, WindowEvent};
pub use winit::window::WindowId;
pub use winit::window::Window as PlatformWindow;

/// Upper bound on a single frame delta, so a stall (drag, breakpoint) does not teleport the camera.
const MAX_DT: f32 = 0.1;

/// Maximum fixed steps per rendered frame; beyond this the backlog is dropped.
const MAX_FIXED_STEPS: u32 = 5;

/// How the mouse gets captured for camera look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GrabMode {
    /// Editor style: hold the right mouse button to look around; the left button stays free
    /// for selecting things. Esc also releases.
    #[default]
    HoldRightButton,
    /// Game style: a left click captures the mouse until Esc releases it.
    ClickLeft,
}

pub struct WindowConfig {
    pub title: String,
    pub logical_size: (f64, f64),
    pub grab_mode: GrabMode,
    /// Fixed simulation rate in Hz (see `App::fixed_update`).
    pub fixed_hz: f64,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Engine".to_owned(),
            logical_size: (1280.0, 720.0),
            grab_mode: GrabMode::default(),
            fixed_hz: 60.0,
        }
    }
}

/// Application callbacks driven by the platform loop.
pub trait App {
    /// Called once, after the OS window exists. The `Arc<Window>` implements
    /// `raw_window_handle::{HasWindowHandle, HasDisplayHandle}`.
    fn init(&mut self, window: Arc<Window>);
    fn resize(&mut self, size: PhysicalSize<u32>);
    /// Simulation step at a constant rate (`WindowConfig::fixed_hz`). Runs zero or more times per
    /// frame, before `update`. Put gameplay and physics here.
    fn fixed_update(&mut self, _dt: f32) {}

    /// Once per rendered frame with the real (clamped) frame time: camera, UI, input handling.
    fn update(&mut self, dt: f32, input: &Input);

    /// `alpha` in [0, 1] is how far render time lies between the last two fixed states.
    fn render(&mut self, alpha: f32);

    /// Sees every window event first (e.g. a GUI layer). `cursor_grabbed` tells whether the mouse
    /// is currently captured by the game view (hidden/locked): a GUI must then ignore pointer and
    /// key presses. Return `true` if the event is captured by the UI: *press* events (key / mouse
    /// button) that are captured never reach `Input` and never start a mouse grab. Releases are
    /// always delivered so no key can stay stuck down. The return value is ignored while grabbed.
    fn window_event(&mut self, _id: WindowId, _event: &WindowEvent, _cursor_grabbed: bool) -> bool {
        false
    }

    /// Polled after every rendered frame; return `true` to close the window and end `run`.
    fn should_exit(&self) -> bool {
        false
    }
}

pub fn run<A: App>(config: WindowConfig, app: A) -> Result<(), winit::error::EventLoopError> {
    let fixed_hz = config.fixed_hz;
    let grab_mode = config.grab_mode;
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut runner = Runner {
        config,
        app,
        window: None,
        input: Input::default(),
        last_frame: Instant::now(),
        fixed: FixedTimestep::new(fixed_hz, MAX_FIXED_STEPS),
        grab_mode,
        dropped_seconds: 0.0,
        last_drop_warning: Instant::now(),
    };
    event_loop.run_app(&mut runner)
}

struct Runner<A: App> {
    config: WindowConfig,
    app: A,
    window: Option<Arc<Window>>,
    input: Input,
    last_frame: Instant,
    fixed: FixedTimestep,
    grab_mode: GrabMode,
    /// Simulation time dropped since the last warning (warnings are limited to 1 per second).
    dropped_seconds: f64,
    last_drop_warning: Instant,
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
            log::debug!("cursor grab {}", if ok { "acquired" } else { "failed" });
            window.set_cursor_visible(!ok);
            self.input.set_cursor_grabbed(ok);
        } else {
            log::debug!("cursor grab released");
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
        let grabbed = self.input.cursor_grabbed();
        let captured = self.app.window_event(id, &event, grabbed) && !grabbed;
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.app.resize(size),
            WindowEvent::Focused(false) => {
                self.input.clear_all();
                self.set_grab(false);
            }
            WindowEvent::MouseInput { state, button, .. } => match (self.grab_mode, button, state) {
                (GrabMode::HoldRightButton, MouseButton::Right, ElementState::Pressed) if !captured && !grabbed => {
                    self.set_grab(true)
                }
                // Releases are never captured by the UI, so the grab cannot get stuck.
                (GrabMode::HoldRightButton, MouseButton::Right, ElementState::Released) if grabbed => {
                    self.set_grab(false)
                }
                (GrabMode::ClickLeft, MouseButton::Left, ElementState::Pressed) if !captured && !grabbed => {
                    self.set_grab(true)
                }
                _ => {}
            },
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
                let plan = self.fixed.advance(f64::from(dt));
                self.dropped_seconds += plan.dropped_seconds;
                if self.dropped_seconds > 0.0 && self.last_drop_warning.elapsed().as_secs() >= 1 {
                    log::warn!("simulation fell behind: dropped {:.0} ms in the last second", self.dropped_seconds * 1000.0);
                    self.dropped_seconds = 0.0;
                    self.last_drop_warning = now;
                }
                let step = self.fixed.step_seconds();
                for _ in 0..plan.steps {
                    self.app.fixed_update(step);
                }
                self.app.update(dt, &self.input);
                self.app.render(plan.alpha);
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
