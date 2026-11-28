use glam::Mat4;

use crate::{Aabb, Ray};

/// Distance along `ray` to the surface of a unit cube placed by `model` (any affine transform:
/// rotation, non-uniform scale, translation). `0.0` if the ray starts inside. `None` on a miss
/// or if `model` is singular.
///
/// The ray is moved into the cube's local space instead of building a rotated box; the ray
/// parameter `t` is unchanged by that affine map, so it is directly a world-space distance.
pub fn intersect_unit_cube(ray: &Ray, model: &Mat4) -> Option<f32> {
    let inverse = model.inverse();
    if !inverse.is_finite() {
        return None;
    }
    let origin = inverse.transform_point3(ray.origin);
    let dir = inverse.transform_vector3(ray.dir);
    Aabb::UNIT_CUBE.intersect(origin, dir).map(|(t_enter, _)| t_enter)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubeHit {
    /// Position of the hit cube in the iterator that was passed in.
    pub index: usize,
    pub t: f32,
}

/// Nearest hit among unit cubes given by their model matrices. Linear scan: meant for
/// interactive picking (one cast per click), not for per-frame bulk queries.
pub fn raycast_unit_cubes<'a>(ray: &Ray, models: impl IntoIterator<Item = &'a Mat4>) -> Option<CubeHit> {
    models
        .into_iter()
        .enumerate()
        .filter_map(|(index, m)| intersect_unit_cube(ray, m).map(|t| CubeHit { index, t }))
        .min_by(|a, b| a.t.total_cmp(&b.t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    fn down_z(x: f32, y: f32) -> Ray {
        Ray::new(Vec3::new(x, y, 10.0), Vec3::NEG_Z).unwrap()
    }

    #[test]
    fn translated_scaled_cube_distance_is_world_space() {
        // Cube centred at z = 0 with edge 4: front face at z = 2 -> distance 8 from z = 10.
        let m = Mat4::from_scale_rotation_translation(Vec3::splat(4.0), Quat::IDENTITY, Vec3::ZERO);
        assert!((intersect_unit_cube(&down_z(0.0, 0.0), &m).unwrap() - 8.0).abs() < 1e-4);
    }

    #[test]
    fn non_uniform_scale_changes_the_footprint() {
        let m = Mat4::from_scale(Vec3::new(10.0, 0.2, 1.0)); // wide, thin slab
        assert!(intersect_unit_cube(&down_z(4.0, 0.0), &m).is_some());
        assert!(intersect_unit_cube(&down_z(0.0, 0.5), &m).is_none());
    }

    #[test]
    fn rotation_is_respected() {
        // Unit cube turned 45 degrees about Y: its half-width along X becomes sqrt(2)/2.
        let m = Mat4::from_quat(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4));
        assert!(intersect_unit_cube(&down_z(0.69, 0.0), &m).is_some());
        assert!(intersect_unit_cube(&down_z(0.72, 0.0), &m).is_none());
        // Axis-aligned cube would already miss at 0.6.
        assert!(intersect_unit_cube(&down_z(0.6, 0.0), &Mat4::IDENTITY).is_none());
    }

    #[test]
    fn nearest_of_several_wins_and_index_is_reported() {
        let near = Mat4::from_translation(Vec3::new(0.0, 0.0, 3.0));
        let far = Mat4::from_translation(Vec3::new(0.0, 0.0, -3.0));
        let aside = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let hit = raycast_unit_cubes(&down_z(0.0, 0.0), [&far, &aside, &near]).unwrap();
        assert_eq!(hit.index, 2);
        assert!((hit.t - 6.5).abs() < 1e-4);
        assert!(raycast_unit_cubes(&down_z(20.0, 0.0), [&far, &near]).is_none());
    }

    #[test]
    fn singular_matrix_is_skipped_not_a_panic_or_nan_hit() {
        let flat = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
        assert!(intersect_unit_cube(&down_z(0.0, 0.0), &flat).is_none());
    }

    #[test]
    fn origin_inside_hits_at_zero() {
        let r = Ray::new(Vec3::ZERO, Vec3::X).unwrap();
        assert_eq!(intersect_unit_cube(&r, &Mat4::IDENTITY), Some(0.0));
    }
}
