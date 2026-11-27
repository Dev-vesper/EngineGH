use glam::{Mat4, Quat, Vec3, Vec4};

/// Local-to-world placement (no hierarchy yet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };

    #[inline]
    pub fn from_position(position: Vec3) -> Self {
        Self { position, ..Self::IDENTITY }
    }

    #[inline]
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Decomposes an affine matrix (shear is discarded).
    pub fn from_matrix(m: Mat4) -> Self {
        let (scale, rotation, position) = m.to_scale_rotation_translation();
        Self { position, rotation, scale }
    }

    /// Linear for position and scale, shortest-path slerp for rotation.
    #[inline]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            position: self.position.lerp(other.position, t),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// `Transform` as of the previous fixed step; the render side interpolates from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviousTransform(pub Transform);

/// Draw this entity as the engine's unit cube. `color` is linear RGBA.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderCube {
    pub color: Vec4,
}
