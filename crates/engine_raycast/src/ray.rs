use glam::{Mat4, Vec2, Vec3, Vec4};

/// A half-line `origin + t * dir`, `t >= 0`. `dir` is always normalised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    /// `None` if `dir` has zero or non-finite length.
    pub fn new(origin: Vec3, dir: Vec3) -> Option<Self> {
        let dir = dir.try_normalize()?;
        origin.is_finite().then_some(Self { origin, dir })
    }

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }

    /// Ray through a pixel. `cursor` and `viewport` are in pixels (+y down). `inv_view_proj` is
    /// the inverse of the camera's view-projection matrix. `ndc_depths` are the NDC depths of
    /// two different points along the pixel's ray, the first one closer to the eye; they encode
    /// the projection convention, so this function works for standard and reverse-Z alike
    /// (`engine_camera::FlyCamera::screen_ray` passes the right values).
    pub fn from_screen(cursor: Vec2, viewport: Vec2, inv_view_proj: Mat4, ndc_depths: (f32, f32)) -> Option<Self> {
        if viewport.x <= 0.0 || viewport.y <= 0.0 {
            return None;
        }
        let ndc = Vec2::new(2.0 * cursor.x / viewport.x - 1.0, 1.0 - 2.0 * cursor.y / viewport.y);
        let unproject = |z: f32| {
            let p = inv_view_proj * Vec4::new(ndc.x, ndc.y, z, 1.0);
            p.truncate() / p.w
        };
        let first = unproject(ndc_depths.0);
        Self::new(first, unproject(ndc_depths.1) - first)
    }
}

/// Distance along `ray` to the plane through `point` with `normal`; `None` if parallel or the
/// plane is behind the origin.
pub fn ray_plane(ray: &Ray, point: Vec3, normal: Vec3) -> Option<f32> {
    let denom = normal.dot(ray.dir);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = normal.dot(point - ray.origin) / denom;
    (t >= 0.0).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_normalises_and_rejects_degenerate() {
        let r = Ray::new(Vec3::ZERO, Vec3::new(0.0, 0.0, -5.0)).unwrap();
        assert!((r.dir.length() - 1.0).abs() < 1e-6);
        assert!(Ray::new(Vec3::ZERO, Vec3::ZERO).is_none());
        assert!(Ray::new(Vec3::splat(f32::NAN), Vec3::X).is_none());
    }

    #[test]
    fn plane_hit_parallel_and_behind() {
        let r = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0)).unwrap();
        assert!((ray_plane(&r, Vec3::ZERO, Vec3::Y).unwrap() - 5.0).abs() < 1e-5);
        let parallel = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::X).unwrap();
        assert!(ray_plane(&parallel, Vec3::ZERO, Vec3::Y).is_none());
        let away = Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::Y).unwrap();
        assert!(ray_plane(&away, Vec3::ZERO, Vec3::Y).is_none());
    }
}
