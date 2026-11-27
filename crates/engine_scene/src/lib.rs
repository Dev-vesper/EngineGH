//! Scene layer: engine components and the systems that connect the ECS world to the renderer.
//!
//! Frame flow: `snapshot_transforms` at the start of every fixed step (so interpolation has the
//! previous state), simulation systems mutate `Transform`, then once per rendered frame
//! `extract_instances` turns the world into plain render data. The renderer never sees the ECS.

mod components;
mod systems;

pub use components::{PreviousTransform, RenderCube, Transform};
pub use hecs::{Entity, World};
pub use systems::{extract_instances, snapshot_transforms, spawn_cube};
