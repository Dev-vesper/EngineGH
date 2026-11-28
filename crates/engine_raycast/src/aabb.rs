use glam::Vec3;

use crate::Ray;

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const UNIT_CUBE: Self = Self { min: Vec3::splat(-0.5), max: Vec3::splat(0.5) };

    /// Slab test. Returns `(t_enter, t_exit)` clipped to `t >= 0` (so `t_enter == 0` when the
    /// origin is inside), or `None` on a miss. Works with an un-normalised direction: the
    /// returned values are then in units of `dir`.
    pub fn intersect(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        let mut t_enter = 0.0_f32;
        let mut t_exit = f32::INFINITY;
        for axis in 0..3 {
            let (o, d) = (origin[axis], dir[axis]);
            let (lo, hi) = (self.min[axis], self.max[axis]);
            if d.abs() < 1e-12 {
                // Parallel to this slab: a miss unless the origin lies between its planes.
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let inv = 1.0 / d;
                let (t0, t1) = ((lo - o) * inv, (hi - o) * inv);
                t_enter = t_enter.max(t0.min(t1));
                t_exit = t_exit.min(t0.max(t1));
                if t_enter > t_exit {
                    return None;
                }
            }
        }
        Some((t_enter, t_exit))
    }

    #[inline]
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, f32)> {
        self.intersect(ray.origin, ray.dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ray(o: [f32; 3], d: [f32; 3]) -> Ray {
        Ray::new(Vec3::from(o), Vec3::from(d)).unwrap()
    }

    #[test]
    fn head_on_hit_reports_entry_and_exit() {
        let (t0, t1) = Aabb::UNIT_CUBE.intersect_ray(&ray([0.0, 0.0, 5.0], [0.0, 0.0, -1.0])).unwrap();
        assert!((t0 - 4.5).abs() < 1e-5 && (t1 - 5.5).abs() < 1e-5);
    }

    #[test]
    fn miss_beside_and_behind() {
        assert!(Aabb::UNIT_CUBE.intersect_ray(&ray([2.0, 0.0, 5.0], [0.0, 0.0, -1.0])).is_none());
        assert!(Aabb::UNIT_CUBE.intersect_ray(&ray([0.0, 0.0, 5.0], [0.0, 0.0, 1.0])).is_none());
    }

    #[test]
    fn origin_inside_enters_at_zero() {
        let (t0, t1) = Aabb::UNIT_CUBE.intersect_ray(&ray([0.0, 0.0, 0.0], [1.0, 0.0, 0.0])).unwrap();
        assert_eq!(t0, 0.0);
        assert!((t1 - 0.5).abs() < 1e-5);
    }

    #[test]
    fn axis_parallel_ray_outside_slab_misses() {
        // Direction has exact zero components: no NaN from 0 * inf.
        assert!(Aabb::UNIT_CUBE.intersect_ray(&ray([0.0, 3.0, 5.0], [0.0, 0.0, -1.0])).is_none());
        assert!(Aabb::UNIT_CUBE.intersect_ray(&ray([0.0, 0.49, 5.0], [0.0, 0.0, -1.0])).is_some());
    }

    #[test]
    fn grazing_diagonal_hit() {
        let r = ray([-2.0, -2.0, 0.0], [1.0, 1.0, 0.0]);
        assert!(Aabb::UNIT_CUBE.intersect_ray(&r).is_some());
    }
}
