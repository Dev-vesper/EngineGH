//! Child-process side: drives one scenario inside a real window and collects measurements.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use engine_camera::FlyCamera;
use engine_memory::{peak_resident_bytes, MemoryStats};
use engine_platform::{App, Input, PlatformWindow};
use engine_render::{Environment, Frame, Renderer, RendererConfig};
use glam::Vec3;

#[cfg(feature = "editor")]
use engine_editor::{Editor, InspectorData};

use crate::report::{AdapterReport, ChildOutput, EnvironmentReport, MemoryReport, Measurements, ScenarioSpec};
use crate::scenario::{self, Scene};
use crate::stats;

/// Hard limit so an occluded / stalled window cannot hang a benchmark forever.
const WALL_LIMIT: Duration = Duration::from_secs(180);

pub type Outcome = Rc<RefCell<Option<Result<ChildOutput, String>>>>;

pub struct BenchApp {
    spec: ScenarioSpec,
    frames: u32,
    warmup: u32,
    outcome: Outcome,

    renderer: Option<Renderer>,
    #[cfg(feature = "editor")]
    editor: Option<Editor>,
    camera: FlyCamera,
    env: Environment,
    scene: Scene,

    /// Presented frames so far, warm-up included. Also the camera-path index.
    rendered: u32,
    skipped: u32,
    prev_start: Option<Instant>,
    frame_ms: Vec<f64>,
    render_ms: Vec<f64>,
    started: Instant,
    done: bool,
}

impl BenchApp {
    pub fn new(spec: ScenarioSpec, frames: u32, warmup: u32, outcome: Outcome) -> Self {
        let scene = scenario::build_scene(spec.instances);
        Self {
            spec,
            frames,
            warmup,
            outcome,
            renderer: None,
            #[cfg(feature = "editor")]
            editor: None,
            camera: FlyCamera::new(Vec3::ZERO, 0.0, 0.0),
            env: Environment::default(),
            scene,
            rendered: 0,
            skipped: 0,
            prev_start: None,
            // Pre-sized so the measurement loop never allocates.
            frame_ms: Vec::with_capacity(frames as usize),
            render_ms: Vec::with_capacity(frames as usize),
            started: Instant::now(),
            done: false,
        }
    }

    fn fail(&mut self, msg: String) {
        *self.outcome.borrow_mut() = Some(Err(msg));
        self.done = true;
    }

    fn finish(&mut self) {
        let Some(r) = self.renderer.as_ref() else { return };
        let info = r.adapter_info();
        let (window_width, window_height) = r.size();
        let environment = EnvironmentReport {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            build_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_owned(),
            window_width,
            window_height,
            adapter: AdapterReport {
                name: info.name.clone(),
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
                driver: info.driver.clone(),
                driver_info: info.driver_info.clone(),
            },
        };

        let duration_s = self.frame_ms.iter().sum::<f64>() / 1000.0;
        let mem = MemoryStats::now();
        let measurements = Measurements {
            effective_msaa_samples: r.sample_count(),
            warmup_frames: self.warmup,
            frames: self.frame_ms.len() as u32,
            skipped_frames: self.skipped,
            duration_s,
            fps_avg: self.frame_ms.len() as f64 / duration_s,
            fps_1pct_low: stats::fps_1pct_low(&self.frame_ms),
            frame_interval_ms: stats::distribution(&self.frame_ms),
            render_call_ms: stats::distribution(&self.render_ms),
            memory: MemoryReport {
                heap_end_bytes: mem.heap_bytes,
                heap_peak_bytes: mem.heap_peak_bytes,
                alloc_count: mem.alloc_count,
                rss_end_bytes: mem.rss_bytes,
                rss_peak_bytes: peak_resident_bytes(),
            },
        };
        *self.outcome.borrow_mut() = Some(Ok(ChildOutput { environment, measurements }));
        self.done = true;
    }
}

impl App for BenchApp {
    fn init(&mut self, window: Arc<PlatformWindow>) {
        let size = window.inner_size();
        #[cfg(feature = "editor")]
        let editor_window = window.clone();
        let config = RendererConfig { vsync: self.spec.vsync, msaa_samples: self.spec.msaa_samples };
        let renderer = match Renderer::new(window, (size.width, size.height), config) {
            Ok(r) => r,
            Err(e) => return self.fail(format!("renderer init failed: {e}")),
        };

        if self.spec.gui {
            #[cfg(feature = "editor")]
            match Editor::new(
                editor_window,
                renderer.device().clone(),
                renderer.queue().clone(),
                renderer.surface_format(),
                renderer.adapter_info(),
            ) {
                Ok(e) => self.editor = Some(e),
                Err(e) => return self.fail(e.to_string()),
            }
            #[cfg(not(feature = "editor"))]
            return self.fail("--gui on requires building with --features editor".to_owned());
        }
        self.renderer = Some(renderer);
        self.started = Instant::now();
    }

    fn resize(&mut self, size: engine_platform::PhysicalSize<u32>) {
        if let Some(r) = self.renderer.as_mut() {
            r.resize(size.width, size.height);
        }
    }

    fn update(&mut self, _dt: f32, _input: &Input) {
        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            editor.update_stats(_dt);
        }
    }

    fn render(&mut self) {
        if self.done {
            return;
        }
        if self.started.elapsed() > WALL_LIMIT {
            return self.fail(format!(
                "timed out after {WALL_LIMIT:?} ({} frames presented, {} skipped; is the window occluded?)",
                self.rendered, self.skipped
            ));
        }
        let Some(r) = self.renderer.as_mut() else { return };

        let (pos, yaw, pitch) = scenario::camera_pose(self.rendered, self.scene.extent);
        self.camera.position = pos;
        self.camera.yaw = yaw;
        self.camera.pitch = pitch;

        let frame = Frame {
            view_proj: self.camera.view_proj(r.aspect()),
            eye: self.camera.position,
            env: self.env,
            instances: &self.scene.instances,
        };

        let t0 = Instant::now();
        #[cfg(feature = "editor")]
        let presented = if let Some(editor) = self.editor.as_mut() {
            let mut data = InspectorData {
                camera: &mut self.camera,
                env: &mut self.env,
                instance_count: self.scene.instances.len(),
            };
            r.render_with_overlay(&frame, Some(&mut |pass, extent| editor.draw(&mut data, pass, extent)))
        } else {
            r.render(&frame)
        };
        #[cfg(not(feature = "editor"))]
        let presented = r.render(&frame);
        let t1 = Instant::now();

        if !presented {
            self.skipped += 1;
            return;
        }
        // Frame `warmup` is the boundary and is excluded entirely; its start anchors the next interval.
        if self.rendered > self.warmup {
            if let Some(prev) = self.prev_start {
                self.frame_ms.push(t0.duration_since(prev).as_secs_f64() * 1000.0);
                self.render_ms.push(t1.duration_since(t0).as_secs_f64() * 1000.0);
            }
        }
        self.prev_start = Some(t0);
        self.rendered += 1;

        if self.frame_ms.len() as u32 >= self.frames {
            self.finish();
        }
    }

    fn should_exit(&self) -> bool {
        self.done
    }
}
