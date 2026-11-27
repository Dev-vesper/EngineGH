//! Glue between `engine_platform::App` and a [`Game`]: renderer, editor, ECS extraction.

use std::fmt;
use std::sync::Arc;

use engine_platform::{App, Input, PhysicalSize, PlatformWindow, WindowEvent, WindowId};
use engine_render::{Frame, Instance, Renderer, RendererConfig};
use engine_scene::{extract_instances, snapshot_transforms, Entity};

#[cfg(feature = "editor")]
use engine_editor::{Editor, EditorOutput, InspectorData, SceneOp};
#[cfg(feature = "editor")]
use engine_scene::{spawn_cube, PreviousTransform, RenderCube, Transform};

use crate::{AppConfig, Game, GameContext};

#[derive(Debug)]
pub struct RunError(String);

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RunError {}

pub(crate) fn run<G: Game>(config: AppConfig, game: G) -> Result<(), RunError> {
    let AppConfig { window, renderer, camera, environment } = config;
    let mut ctx = GameContext { world: engine_scene::World::new(), camera, env: environment, quit: false };
    let mut game = game;
    game.setup(&mut ctx);

    let runtime = Runtime {
        game,
        ctx,
        initial_camera: camera,
        renderer_config: renderer,
        renderer: None,
        instances: Vec::new(),
        entities: Vec::new(),
        #[cfg(feature = "editor")]
        editor: None,
    };
    engine_platform::run(window, runtime).map_err(|e| RunError(e.to_string()))
}

struct Runtime<G: Game> {
    game: G,
    ctx: GameContext,
    #[cfg_attr(not(feature = "editor"), allow(dead_code))]
    initial_camera: engine_camera::FlyCamera,
    renderer_config: RendererConfig,
    renderer: Option<Renderer>,
    /// Render data of the current frame; `entities[i]` is the entity behind `instances[i]`.
    instances: Vec<Instance>,
    entities: Vec<Entity>,
    #[cfg(feature = "editor")]
    editor: Option<Editor>,
}

#[cfg(feature = "editor")]
impl<G: Game> Runtime<G> {
    /// Applies what the editor asked for during the frame that just rendered. Scene indices
    /// refer to this frame's `entities`.
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
            self.ctx.camera = self.initial_camera;
        }
        if out.quit {
            self.ctx.request_quit();
        }

        let entity_at = |i: usize| self.entities.get(i).copied();
        let world = &mut self.ctx.world;
        match out.scene {
            SceneOp::None => {}
            SceneOp::Add { model, color } => {
                spawn_cube(world, Transform::from_matrix(model), RenderCube { color });
            }
            SceneOp::Duplicate(i) => {
                let source = entity_at(i)
                    .and_then(|e| world.query_one_mut::<(&Transform, &RenderCube)>(e).ok())
                    .map(|(t, c)| (*t, *c));
                if let Some((mut transform, cube)) = source {
                    transform.position += glam::Vec3::X; // visible offset from the original
                    spawn_cube(world, transform, cube);
                }
            }
            SceneOp::Remove(i) => {
                if let Some(e) = entity_at(i) {
                    let _ = world.despawn(e);
                }
            }
            SceneOp::Set { index, model, color } => {
                if let Some(e) = entity_at(index) {
                    if let Ok((t, prev, c)) =
                        world.query_one_mut::<(&mut Transform, &mut PreviousTransform, &mut RenderCube)>(e)
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

impl<G: Game> App for Runtime<G> {
    fn init(&mut self, window: Arc<PlatformWindow>) {
        let size = window.inner_size();
        #[cfg(feature = "editor")]
        let editor_window = window.clone();
        let renderer = match Renderer::new(window, (size.width, size.height), self.renderer_config) {
            Ok(r) => r,
            Err(e) => {
                log::error!("renderer init failed: {e}");
                self.ctx.request_quit();
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
        self.ctx.quit_requested()
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(r) = self.renderer.as_mut() {
            r.resize(size.width, size.height);
        }
    }

    fn fixed_update(&mut self, dt: f32) {
        // Must precede every system that moves a Transform: interpolation needs the old state.
        snapshot_transforms(&mut self.ctx.world);
        self.game.fixed_update(&mut self.ctx, dt);
    }

    fn update(&mut self, dt: f32, input: &Input) {
        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            editor.update_stats(dt);
        }
        self.game.update(&mut self.ctx, dt, input);
    }

    fn render(&mut self, alpha: f32) {
        extract_instances(&self.ctx.world, alpha, &mut self.instances, &mut self.entities);

        let Some(r) = self.renderer.as_mut() else { return };
        #[cfg(feature = "editor")]
        let status = r.status();
        let frame = Frame {
            view_proj: self.ctx.camera.view_proj(r.aspect()),
            eye: self.ctx.camera.position,
            env: self.ctx.env,
            instances: &self.instances,
        };

        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            let mut data = InspectorData {
                camera: &mut self.ctx.camera,
                env: &mut self.ctx.env,
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
