use std::sync::Arc;

#[cfg(feature = "editor")]
use engine_camera::FlyCamera;
use engine_platform::{App, Input, PhysicalSize, PlatformWindow, WindowEvent, WindowId};
#[cfg(feature = "editor")]
use engine_render::Environment;
use engine_render::{Frame, Instance, Renderer, RendererConfig};
use engine_scene::{extract_instances, snapshot_transforms, Entity};

#[cfg(feature = "editor")]
use engine_editor::{Editor, EditorOutput, InspectorData};

use crate::{Engine, Game, GameConfig};

/// Bridges a [`Game`] to the platform loop.
pub(crate) struct EngineApp<G: Game> {
    pub(crate) game: G,
    pub(crate) engine: Engine,
    renderer_config: RendererConfig,
    /// State after `Game::setup`; what the editor's reset restores.
    #[cfg(feature = "editor")]
    pub(crate) initial_camera: FlyCamera,
    #[cfg(feature = "editor")]
    pub(crate) initial_env: Environment,
    pub(crate) renderer: Option<Renderer>,
    /// Render data of the current frame; `entities[i]` is the entity behind `instances[i]`.
    pub(crate) instances: Vec<Instance>,
    pub(crate) entities: Vec<Entity>,
    #[cfg(feature = "editor")]
    editor: Option<Editor>,
}

impl<G: Game> EngineApp<G> {
    pub(crate) fn new(mut game: G, config: GameConfig) -> Self {
        let mut engine = Engine::new();
        game.setup(&mut engine);
        Self {
            #[cfg(feature = "editor")]
            initial_camera: engine.camera,
            #[cfg(feature = "editor")]
            initial_env: engine.env,
            game,
            engine,
            renderer_config: config.renderer,
            renderer: None,
            instances: Vec::new(),
            entities: Vec::new(),
            #[cfg(feature = "editor")]
            editor: None,
        }
    }
}

impl<G: Game> App for EngineApp<G> {
    fn init(&mut self, window: Arc<PlatformWindow>) {
        let size = window.inner_size();
        #[cfg(feature = "editor")]
        let editor_window = window.clone();
        let renderer = match Renderer::new(window, (size.width, size.height), self.renderer_config) {
            Ok(r) => r,
            Err(e) => {
                log::error!("renderer init failed: {e}");
                self.engine.quit = true;
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
        self.engine.quit
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(r) = self.renderer.as_mut() {
            r.resize(size.width, size.height);
        }
    }

    fn fixed_update(&mut self, dt: f32) {
        if !self.engine.sim.should_step() {
            return;
        }
        snapshot_transforms(&mut self.engine.world); // before anything moves a Transform
        let scaled = dt * self.engine.sim.time_scale;
        self.game.fixed_update(&mut self.engine, scaled);
    }

    fn update(&mut self, dt: f32, input: &Input) {
        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            editor.update_stats(dt);
        }
        self.game.update(&mut self.engine, input, dt);
    }

    fn render(&mut self, alpha: f32) {
        // While paused no new states arrive: show the current one instead of oscillating.
        let alpha = if self.engine.sim.paused { 1.0 } else { alpha };
        extract_instances(&self.engine.world, alpha, &mut self.instances, &mut self.entities);

        let Some(r) = self.renderer.as_mut() else { return };
        #[cfg(feature = "editor")]
        let status = r.status();
        let frame = Frame {
            view_proj: self.engine.camera.view_proj(r.aspect()),
            eye: self.engine.camera.position,
            env: self.engine.env,
            instances: &self.instances,
        };

        #[cfg(feature = "editor")]
        if let Some(editor) = self.editor.as_mut() {
            let mut data = InspectorData {
                camera: &mut self.engine.camera,
                env: &mut self.engine.env,
                sim: &mut self.engine.sim,
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
