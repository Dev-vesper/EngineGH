use engine_render::Instance;
use hecs::{Entity, World};

use crate::components::{PreviousTransform, RenderCube, Transform};

/// Spawns a renderable cube. `PreviousTransform` starts equal to `transform`, so a new entity
/// does not interpolate in from the origin.
pub fn spawn_cube(world: &mut World, transform: Transform, cube: RenderCube) -> Entity {
    world.spawn((transform, PreviousTransform(transform), cube))
}

/// Run at the start of every fixed step, before simulation systems change `Transform`.
pub fn snapshot_transforms(world: &mut World) {
    for (transform, previous) in world.query_mut::<(&Transform, &mut PreviousTransform)>() {
        previous.0 = *transform;
    }
}

/// Builds render data for the current frame. `alpha` (from `FixedTimestep`) blends the previous
/// and current simulation state. `entities[i]` is the entity behind `instances[i]`; both
/// vectors are cleared and refilled, so their capacity is reused and steady-state frames do
/// not allocate.
pub fn extract_instances(world: &World, alpha: f32, instances: &mut Vec<Instance>, entities: &mut Vec<Entity>) {
    instances.clear();
    entities.clear();
    for (entity, transform, previous, cube) in world.query::<(Entity, &Transform, &PreviousTransform, &RenderCube)>().iter()
    {
        let blended = previous.0.lerp(transform, alpha);
        instances.push(Instance { model: blended.matrix(), color: cube.color });
        entities.push(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3, Vec4};

    const WHITE: RenderCube = RenderCube { color: Vec4::ONE };

    #[test]
    fn spawned_entity_does_not_interpolate_from_origin() {
        let mut w = World::new();
        spawn_cube(&mut w, Transform::from_position(Vec3::new(10.0, 0.0, 0.0)), WHITE);
        let (mut i, mut e) = (Vec::new(), Vec::new());
        extract_instances(&w, 0.5, &mut i, &mut e);
        assert_eq!(i[0].model.w_axis.x, 10.0);
    }

    #[test]
    fn snapshot_then_move_interpolates_between_states() {
        let mut w = World::new();
        let id = spawn_cube(&mut w, Transform::IDENTITY, WHITE);
        snapshot_transforms(&mut w);
        w.query_one_mut::<&mut Transform>(id).unwrap().position = Vec3::new(10.0, 0.0, 0.0);

        let (mut i, mut e) = (Vec::new(), Vec::new());
        extract_instances(&w, 0.25, &mut i, &mut e);
        assert!((i[0].model.w_axis.x - 2.5).abs() < 1e-5);
        extract_instances(&w, 1.0, &mut i, &mut e);
        assert!((i[0].model.w_axis.x - 10.0).abs() < 1e-5);
        assert_eq!(e[0], id);
    }

    #[test]
    fn extract_reuses_buffers_and_skips_non_renderables() {
        let mut w = World::new();
        spawn_cube(&mut w, Transform::IDENTITY, WHITE);
        w.spawn((Transform::IDENTITY,)); // no RenderCube
        let (mut i, mut e) = (Vec::new(), Vec::new());
        extract_instances(&w, 0.0, &mut i, &mut e);
        let cap = i.capacity();
        extract_instances(&w, 0.0, &mut i, &mut e);
        assert_eq!((i.len(), e.len(), i.capacity()), (1, 1, cap));
    }

    #[test]
    fn rotation_interpolates_along_shortest_path() {
        let a = Transform::IDENTITY;
        let b = Transform { rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2), ..a };
        let mid = a.lerp(&b, 0.5).rotation;
        assert!(mid.angle_between(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)) < 1e-4);
    }

    #[test]
    fn matrix_round_trip() {
        let t = Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::from_rotation_x(0.7),
            scale: Vec3::new(2.0, 1.0, 0.5),
        };
        let r = Transform::from_matrix(t.matrix());
        assert!((r.position - t.position).length() < 1e-5);
        assert!((r.scale - t.scale).length() < 1e-5);
        assert!(r.rotation.angle_between(t.rotation) < 1e-4);
    }
}
