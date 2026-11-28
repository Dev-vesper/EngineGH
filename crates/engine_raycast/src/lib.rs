//! Ray casting toolbox. Pure math on `glam` types: no ECS, no renderer, no windowing, so any
//! subsystem (editor picking, physics queries, AI line of sight, tools) can use it.

mod aabb;
mod cubes;
mod ray;

pub use aabb::Aabb;
pub use cubes::{intersect_unit_cube, raycast_unit_cubes, CubeHit};
pub use ray::{ray_plane, Ray};
