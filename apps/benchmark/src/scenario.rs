//! Deterministic scene and camera path. Nothing here depends on wall-clock time or RNG state,
//! so every run (and every machine) renders exactly the same frames.

use engine_render::Instance;
use glam::{Mat4, Quat, Vec3, Vec4};

const SPACING: f32 = 3.0;
/// Camera orbit step per rendered frame.
const ORBIT_STEP_DEG: f32 = 0.4;

/// Integer hash mapped to [0, 1).
fn hash01(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / 16_777_216.0
}

pub struct Scene {
    pub instances: Vec<Instance>,
    /// Half-width of the populated square, used to frame the camera.
    pub extent: f32,
}

/// `count` cubes on a square lattice with hash-derived size, yaw and colour.
pub fn build_scene(count: u32) -> Scene {
    let side = (count as f32).sqrt().ceil().max(1.0) as u32;
    let half = side as f32 * 0.5;
    let instances = (0..count)
        .map(|i| {
            let (gx, gz) = ((i % side) as f32 - half, (i / side) as f32 - half);
            let size = 0.8 + 0.8 * hash01(i, 1);
            let rot = Quat::from_rotation_y(hash01(i, 2) * std::f32::consts::TAU);
            Instance {
                model: Mat4::from_scale_rotation_translation(
                    Vec3::splat(size),
                    rot,
                    Vec3::new(gx * SPACING, size * 0.5, gz * SPACING),
                ),
                color: Vec4::new(0.2 + 0.8 * hash01(i, 3), 0.2 + 0.8 * hash01(i, 4), 0.2 + 0.8 * hash01(i, 5), 1.0),
            }
        })
        .collect();
    Scene { instances, extent: half * SPACING }
}

/// Camera on a circle around the origin looking at it. Returns (position, yaw, pitch) in the
/// convention of `engine_camera::FlyCamera`.
pub fn camera_pose(frame: u32, extent: f32) -> (Vec3, f32, f32) {
    let angle = frame as f32 * ORBIT_STEP_DEG.to_radians();
    let radius = extent * 0.9 + 10.0;
    let height = extent * 0.35 + 5.0;
    let pos = Vec3::new(angle.cos() * radius, height, angle.sin() * radius);
    let dir = (-pos).normalize();
    (pos, (-dir.x).atan2(-dir.z), dir.y.asin())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_camera::FlyCamera;

    #[test]
    fn scene_is_deterministic_and_sized() {
        let a = build_scene(1000);
        let b = build_scene(1000);
        assert_eq!(a.instances.len(), 1000);
        assert_eq!(a.instances[417].model, b.instances[417].model);
        assert_eq!(a.instances[417].color, b.instances[417].color);
    }

    #[test]
    fn camera_pose_looks_at_origin() {
        for frame in [0, 123, 999] {
            let (pos, yaw, pitch) = camera_pose(frame, 100.0);
            let cam = FlyCamera::new(pos, yaw, pitch);
            let to_origin = (-pos).normalize();
            assert!((cam.forward() - to_origin).length() < 1e-4, "frame {frame}");
        }
    }
}
