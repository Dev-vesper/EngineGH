//! Applies the requests the editor produced during a frame (see `engine_editor::EditorOutput`).

use engine_editor::{EditorOutput, SceneOp};
use engine_scene::{spawn_cube, PreviousTransform, RenderCube, Transform};
use glam::Vec3;

use crate::app::EngineApp;
use crate::Game;

impl<G: Game> EngineApp<G> {
    pub(crate) fn apply_editor_output(&mut self, out: EditorOutput) {
        if let Some(r) = self.renderer.as_mut() {
            if let Some(vsync) = out.vsync {
                r.set_vsync(vsync);
            }
            if let Some(samples) = out.msaa {
                r.set_msaa(samples);
            }
        }
        if out.reset_camera {
            self.engine.camera = self.initial_camera;
        }
        if out.reset_environment {
            self.engine.env = self.initial_env;
        }
        self.engine.quit |= out.quit;

        // Scene indices refer to this frame's `entities` (see `EngineApp::render`).
        let world = &mut self.engine.world;
        let entity_at = |i: usize| self.entities.get(i).copied();
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
                    transform.position += Vec3::X; // visible offset from the original
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
