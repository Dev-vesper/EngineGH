use std::sync::Arc;

use engine_camera::FlyCamera;
use engine_platform::{run, App, Input, Key, PlatformWindow, WindowConfig};
use engine_render::{Frame, Instance, Renderer};
use glam::{Mat4, Quat, Vec3, Vec4};

/// Linear-space sky / fog colour.
const SKY: Vec3 = Vec3::new(0.02, 0.04, 0.09);
const FOG_DENSITY: f32 = 0.012;
const LIGHT_DIR: Vec3 = Vec3::new(0.4, 0.9, 0.3);

struct Sandbox {
    renderer: Option<Renderer>,
    camera: FlyCamera,
    instances: Vec<Instance>,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            renderer: None,
            camera: FlyCamera::new(Vec3::new(0.0, 5.0, 20.0), 0.0, -0.2),
            instances: build_scene(),
        }
    }
}

fn instance(translation: Vec3, scale: Vec3, rotation: Quat, rgb: [f32; 3]) -> Instance {
    Instance {
        model: Mat4::from_scale_rotation_translation(scale, rotation, translation),
        color: Vec4::new(rgb[0], rgb[1], rgb[2], 1.0),
    }
}

fn build_scene() -> Vec<Instance> {
    // Ground slab: top face at y = -0.002 so it never z-fights with the y = 0 grid.
    let mut v = vec![instance(
        Vec3::new(0.0, -0.052, 0.0),
        Vec3::new(400.0, 0.1, 400.0),
        Quat::IDENTITY,
        [0.015, 0.018, 0.022],
    )];

    // Ring of cubes of varying size and tilt.
    const COUNT: usize = 12;
    for i in 0..COUNT {
        let t = i as f32 / COUNT as f32;
        let angle = t * std::f32::consts::TAU;
        let radius = 8.0;
        let size = 0.8 + 1.4 * (0.5 + 0.5 * (angle * 3.0).sin());
        let rot = Quat::from_rotation_y(angle) * Quat::from_rotation_x(0.35 * (angle * 2.0).cos());
        let pos = Vec3::new(angle.cos() * radius, size * 0.5 + 0.01, angle.sin() * radius);
        let hue = [0.25 + 0.6 * t, 0.35 + 0.4 * (1.0 - t), 0.8 - 0.5 * t];
        v.push(instance(pos, Vec3::splat(size), rot, hue));
    }

    // Central tower (non-uniform scale exercises the normal matrix).
    v.push(instance(Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.5, 4.0, 1.5), Quat::IDENTITY, [0.7, 0.55, 0.2]));
    v
}

impl App for Sandbox {
    fn init(&mut self, window: Arc<PlatformWindow>) {
        let size = window.inner_size();
        match Renderer::new(window, (size.width, size.height)) {
            Ok(r) => self.renderer = Some(r),
            Err(e) => log::error!("renderer init failed: {e}"),
        }
    }

    fn resize(&mut self, size: engine_platform::PhysicalSize<u32>) {
        if let Some(r) = self.renderer.as_mut() {
            r.resize(size.width, size.height);
        }
    }

    fn update(&mut self, dt: f32, input: &Input) {
        if input.cursor_grabbed() {
            self.camera.look(input.mouse_delta());
        }
        let axes = Vec3::new(
            input.axis(Key::KeyD, Key::KeyA),
            input.axis(Key::Space, Key::ControlLeft),
            input.axis(Key::KeyW, Key::KeyS),
        );
        let boost = if input.is_down(Key::ShiftLeft) { 3.0 } else { 1.0 };
        self.camera.translate(axes, boost, dt);
    }

    fn render(&mut self) {
        let Some(r) = self.renderer.as_mut() else { return };
        let frame = Frame {
            view_proj: self.camera.view_proj(r.aspect()),
            eye: self.camera.position,
            light_dir: LIGHT_DIR,
            sky: SKY,
            fog_density: FOG_DENSITY,
            instances: &self.instances,
        };
        r.render(&frame);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config = WindowConfig { title: "Engine Sandbox".to_owned(), ..Default::default() };
    if let Err(e) = run(config, Sandbox::new()) {
        log::error!("event loop error: {e}");
    }
}
