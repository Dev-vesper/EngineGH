//! Free-fly camera. Depends only on `glam`; knows nothing about windowing or GPU.

use glam::camera::rh::{proj::directx, view};
use engine_raycast::Ray;
use glam::{Mat4, Vec2, Vec3};

const WORLD_UP: Vec3 = Vec3::Y;
/// Pitch is clamped just below +-90 degrees to keep the basis well-defined.
const MAX_PITCH: f32 = 89.0_f32 * std::f32::consts::PI / 180.0;

#[derive(Clone, Copy, Debug)]
pub struct Projection {
    pub fov_y: f32,
    pub near: f32,
}

impl Default for Projection {
    fn default() -> Self {
        Self { fov_y: 60.0_f32.to_radians(), near: 0.1 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlyCamera {
    pub position: Vec3,
    /// Rotation around world Y, radians. 0 looks along -Z, positive turns left.
    pub yaw: f32,
    /// Rotation around camera right axis, radians. Positive looks up.
    pub pitch: f32,
    pub projection: Projection,
    /// Metres per second.
    pub move_speed: f32,
    /// Radians per pixel of mouse motion.
    pub look_sensitivity: f32,
}

impl FlyCamera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch: pitch.clamp(-MAX_PITCH, MAX_PITCH),
            projection: Projection::default(),
            move_speed: 6.0,
            look_sensitivity: 0.0022,
        }
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        self.forward().cross(WORLD_UP).normalize()
    }

    /// Applies mouse look. `delta` is in pixels, screen space (+x right, +y down).
    pub fn look(&mut self, delta: Vec2) {
        self.yaw -= delta.x * self.look_sensitivity;
        self.pitch = (self.pitch - delta.y * self.look_sensitivity).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// `axes`: x = strafe (+right), y = vertical (+up, world space), z = forward (+forward).
    /// `boost` multiplies speed.
    pub fn translate(&mut self, axes: Vec3, boost: f32, dt: f32) {
        let wish = self.right() * axes.x + WORLD_UP * axes.y + self.forward() * axes.z;
        if wish.length_squared() > 0.0 {
            self.position += wish.normalize() * self.move_speed * boost * dt;
        }
    }

    #[inline]
    pub fn view(&self) -> Mat4 {
        view::look_to_mat4(self.position, self.forward(), WORLD_UP)
    }

    /// Right-handed view space, NDC Y-up (wgpu), infinite far plane, reverse-Z:
    /// depth 1.0 at `near`, 0.0 at infinity.
    /// Pair with a float depth buffer, clear value 0.0 and `CompareFunction::GreaterEqual`
    /// for near-uniform depth precision across the whole range.
    #[inline]
    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        let p = &self.projection;
        directx::perspective_infinite_reverse(p.fov_y, aspect, p.near)
    }

    #[inline]
    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view()
    }

    /// World-space ray through a pixel (`cursor` in pixels, +y down, inside `viewport`).
    /// Reverse-Z: NDC depth 1.0 is the near plane, 0.5 lies at twice the near distance.
    pub fn screen_ray(&self, cursor: Vec2, viewport: Vec2) -> Option<Ray> {
        let inv = self.view_proj(viewport.x / viewport.y).inverse();
        Ray::from_screen(cursor, viewport, inv, (1.0, 0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn default_orientation_looks_down_negative_z() {
        let c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        assert!(close(c.forward(), Vec3::NEG_Z));
        assert!(close(c.right(), Vec3::X));
    }

    #[test]
    fn mouse_right_turns_right_and_up_looks_up() {
        let mut c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        c.look(Vec2::new(100.0, 0.0));
        assert!(c.forward().x > 0.0);
        let mut c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        c.look(Vec2::new(0.0, -100.0));
        assert!(c.forward().y > 0.0);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        c.look(Vec2::new(0.0, -1.0e6));
        assert!(c.pitch <= MAX_PITCH && c.forward().is_finite());
    }

    #[test]
    fn reverse_z_maps_near_to_one_and_far_to_zero() {
        let c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        let vp = c.view_proj(16.0 / 9.0);
        let depth = |z: f32| {
            let p = vp * Vec3::new(0.0, 0.0, -z).extend(1.0);
            p.z / p.w
        };
        assert!((depth(c.projection.near) - 1.0).abs() < 1e-5);
        assert!(depth(1.0e6) < 1e-6);
        assert!(depth(10.0) > depth(100.0)); // monotonic: closer = larger
    }

    #[test]
    fn translation_is_speed_independent_of_direction_mix() {
        let mut c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        c.translate(Vec3::new(1.0, 0.0, 1.0), 1.0, 1.0);
        assert!((c.position.length() - c.move_speed).abs() < 1e-4);
    }

    #[test]
    fn screen_ray_through_centre_is_the_view_direction() {
        let c = FlyCamera::new(Vec3::new(1.0, 2.0, 3.0), 0.7, -0.3);
        let r = c.screen_ray(Vec2::new(640.0, 360.0), Vec2::new(1280.0, 720.0)).unwrap();
        assert!((r.dir - c.forward()).length() < 1e-4);
        // Starts on the near plane, directly in front of the eye.
        assert!(((r.origin - c.position).length() - c.projection.near).abs() < 1e-4);
    }

    #[test]
    fn screen_ray_edge_angle_matches_the_field_of_view() {
        let c = FlyCamera::new(Vec3::ZERO, 0.0, 0.0);
        // Top-centre pixel: half the vertical FOV above the forward axis.
        let r = c.screen_ray(Vec2::new(640.0, 0.0), Vec2::new(1280.0, 720.0)).unwrap();
        let angle = r.dir.angle_between(c.forward());
        assert!((angle - c.projection.fov_y * 0.5).abs() < 1e-3, "angle {angle}");
        assert!(r.dir.y > 0.0, "pixel row 0 is the top of the screen");
        // Right edge, vertical centre: horizontal half-FOV is atan(tan(fov/2) * aspect).
        let r = c.screen_ray(Vec2::new(1280.0, 360.0), Vec2::new(1280.0, 720.0)).unwrap();
        assert!(r.dir.x > 0.0);
        let expected = ((c.projection.fov_y * 0.5).tan() * (1280.0 / 720.0)).atan();
        assert!((r.dir.angle_between(c.forward()) - expected).abs() < 1e-3);
    }
}
