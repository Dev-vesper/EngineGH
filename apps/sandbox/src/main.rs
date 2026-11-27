use std::sync::Arc;

use engine_camera::FlyCamera;
use engine_platform::{run, App, Input, Key, PlatformWindow, WindowConfig, WindowEvent, WindowId};
use engine_render::{Environment, Frame, Instance, Renderer, RendererConfig};
use engine_scene::{extract_instances, snapshot_transforms, spawn_cube, Entity, RenderCube, Transform, World};
use glam::{Quat, Vec3, Vec4};

#[cfg(feature = "editor")]
use engine_editor::{Editor, EditorOutput, InspectorData, SceneOp};
#[cfg(feature = "editor")]
use engine_scene::PreviousTransform;

#[cfg(feature = "editor")]
#[global_allocator]
static ALLOCATOR: engine_memory::TrackingAllocator = engine_memory::TrackingAllocator;

/// Angular velocity around world Y in rad/s. Sandbox-specific demo component.
struct Spin(f32);

struct Sandbox {
    renderer: Option<Renderer>,
    camera: FlyCamera,
    env: Environment,
    world: World,
    /// Render data of the current frame; `entities[i]` is the entity behind `instances[i]`.
    instances: Vec<Instance>,
    entities: Vec<Entity>,
    quit: bool,
    #[cfg(feature = "editor")]
    editor: Option<Editor>,
}

impl Sandbox {
    fn new() -> Self {
        let mut world = World::new();
        populate(&mut world);
        Self {
            renderer: None,
            camera: default_camera(),
            env: Environment::default(),
            world,
            instances: Vec::new(),
            entities: Vec::new(),
            quit: false,
            #[cfg(feature = "editor")]
            editor: None,
        }
    }
}

fn default_camera() -> FlyCamera {
    FlyCamera::new(Vec3::new(0.0, 5.0, 20.0), 0.0, -0.2)
}

fn cube(world: &mut World, position: Vec3, scale: Vec3, rotation: Quat, rgb: [f32; 3]) -> Entity {
    spawn_cube(
        world,
        Transform { position, rotation, scale },
        RenderCube { color: Vec4::new(rgb[0], rgb[1], rgb[2], 1.0) },
    )
}

fn populate(world: &mut World) {
    // Ground slab: top face at y = -0.002 so it never z-fights with the y = 0 grid.
    cube(world, Vec3::new(0.0, -0.052, 0.0), Vec3::new(400.0, 0.1, 400.0), Quat::IDENTITY, [0.015, 0.018, 0.022]);

    // Ring of spinning cubes of varying size and tilt.
    const COUNT: usize = 12;
    for i in 0..COUNT {
        let t = i as f32 / COUNT as f32;
        let angle = t * std::f32::consts::TAU;
        let size = 0.8 + 1.4 * (0.5 + 0.5 * (angle * 3.0).sin());
        let rot = Quat::from_rotation_y(angle) * Quat::from_rotation_x(0.35 * (angle * 2.0).cos());
        let pos = Vec3::new(angle.cos() * 8.0, size * 0.5 + 0.01, angle.sin() * 8.0);
        let rgb = [0.25 + 0.6 * t, 0.35 + 0.4 * (1.0 - t), 0.8 - 0.5 * t];
        let e = cube(world, pos, Vec3::splat(size), rot, rgb);
        let direction = if i % 2 == 0 { 1.0 } else { -1.0 };
        world.insert_one(e, Spin(0.6 * direction)).expect("entity was just spawned");
    }

    // Central tower (non-uniform scale exercises the normal matrix).
    cube(world, Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.5, 4.0, 1.5), Quat::IDENTITY, [0.7, 0.55, 0.2]);
}

/// Simulation system: runs in `fixed_update`, so it is frame-rate independent.
fn spin_system(world: &mut World, dt: f32) {
    for (transform, spin) in world.query_mut::<(&mut Transform, &Spin)>() {
        transform.rotation = (Quat::from_rotation_y(spin.0 * dt) * transform.rotation).normalize();
    }
}

#[cfg(feature = "editor")]
impl Sandbox {
    /// Applies what the editor asked for during the frame that just rendered.
    fn apply_editor_output(&mut self, out: EditorOutput) {
        if let Some(r) = self.renderer.as_mut() {
            if let Some(vsync) = out.vsync {
                r.set_vsync(vsync);
            }
            if let Some(samples) = out.msaa {
                r.set_msaa(samples);
            }
        }
        if out.reset_camera {
            self.camera = default_camera();
        }
        self.quit |= out.quit;

        // Scene indices refer to this frame's `entities` (see `render`).
        let entity_at = |i: usize| self.entities.get(i).copied();
        match out.scene {
            SceneOp::None => {}
            SceneOp::Add { model, color } => {
                spawn_cube(&mut self.world, Transform::from_matrix(model), RenderCube { color });
            }
            SceneOp::Duplicate(i) => {
                let source = entity_at(i)
                    .and_then(|e| self.world.query_one_mut::<(&Transform, &RenderCube)>(e).ok())
                    .map(|(t, c)| (*t, *c));
                if let Some((mut transform, cube)) = source {
                    transform.position += Vec3::X; // visible offset from the original
                    spawn_cube(&mut self.world, transform, cube);
                }
            }
            SceneOp::Remove(i) => {
                if let Some(e) = entity_at(i) {
                    let _ = self.world.despawn(e);
                }
            }
            SceneOp::Set { index, model, color } => {
                if let Some(e) = entity_at(index) {
                    if let Ok((t, prev, c)) =
                        self.world.query_one_mut::<(&mut Transform, &mut PreviousTransform, &mut RenderCube)>(e)
                    {
                        *t = Transform::from_matrix(model);
                        prev.0 = *t; // an edit is a teleport, not motion to interpolate
                        c.color = color;
                    }
                }
            }
        }
    }
}

impl App for Sandbox {
    fn init(&mut self, window: Arc<PlatformWindow>) {
        let size = window.inner_size();
        #[cfg(feature = "editor")]
        let editor_window = window.clone();
        let renderer = match Renderer::new(window, (size.width, size.height), RendererConfig::default()) {
            Ok(r) => r,
            Err(e) => {
                log::error!("renderer init failed: {e}");
                return;
            }
        };
        #[cfg(feature = "editor")]
        {
            self.editor = Editor::new(
                editor_window,
                renderer.device().clone(),
                renderer.queue().clone(),
                renderer.surface_format(),
                renderer.adapter_info(),
            )
            .map_err(|e| log::error!("{e}"))
            .ok();
        }
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, id: WindowId, event: &WindowEvent, cursor_grabbed: bool) -> bool {
        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            return editor.handle_event(id, event, cursor_grabbed);
        }
        let _ = (id, event, cursor_grabbed);
        false
    }

    fn should_exit(&self) -> bool {
        self.quit
    }

    fn resize(&mut self, size: engine_platform::PhysicalSize<u32>) {
        if let Some(r) = self.renderer.as_mut() {
            r.resize(size.width, size.height);
        }
    }

    fn fixed_update(&mut self, dt: f32) {
        snapshot_transforms(&mut self.world); // must precede every system that moves a Transform
        spin_system(&mut self.world, dt);
    }

    fn update(&mut self, dt: f32, input: &Input) {
        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            editor.update_stats(dt);
        }
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

    fn render(&mut self, alpha: f32) {
        extract_instances(&self.world, alpha, &mut self.instances, &mut self.entities);

        let Some(r) = self.renderer.as_mut() else { return };
        #[cfg(feature = "editor")]
        let status = r.status();
        let frame = Frame {
            view_proj: self.camera.view_proj(r.aspect()),
            eye: self.camera.position,
            env: self.env,
            instances: &self.instances,
        };

        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            let mut data = InspectorData {
                camera: &mut self.camera,
                env: &mut self.env,
                instances: &self.instances,
                render: status,
            };
            let mut out = EditorOutput::default();
            r.render_with_overlay(&frame, Some(&mut |pass, extent| editor.draw(&mut data, &mut out, pass, extent)));
            self.apply_editor_output(out);
            return;
        }
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
