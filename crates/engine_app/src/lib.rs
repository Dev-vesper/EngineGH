//! Game framework layer: owns the window, renderer, editor overlay, ECS world and the frame
//! loop, so a game only implements [`Game`]. Everything here is plain source in this repository;
//! a game that needs something different can bypass this crate and use `engine_platform`,
//! `engine_render` and the others directly (see `apps/benchmark` for an example).

mod app;
mod controls;
#[cfg(feature = "editor")]
mod editor_bridge;

pub use controls::fly_camera;

/// Everything a game normally needs, in one import.
pub mod prelude {
    pub use crate::{fly_camera, run, Engine, Game, GameConfig};
    pub use engine_camera::FlyCamera;
    pub use engine_platform::{Input, Key};
    pub use engine_render::{Environment, RendererConfig};
    pub use engine_scene::{
        spawn_cube, Entity, PreviousTransform, RenderCube, Transform, World,
    };
    pub use engine_time::SimControl;
    pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
}

use engine_camera::FlyCamera;
use engine_render::{Environment, RendererConfig};
use engine_scene::World;
use engine_time::SimControl;
use glam::Vec3;

/// Installs the heap-tracking allocator (shown in the editor's Memory panel) when the `editor`
/// feature is on. Put `engine_app::track_memory!();` once at the root of the game binary.
#[cfg(feature = "editor")]
#[macro_export]
macro_rules! track_memory {
    () => {
        #[global_allocator]
        static ENGINE_ALLOCATOR: $crate::TrackingAllocator = $crate::TrackingAllocator;
    };
}

#[cfg(not(feature = "editor"))]
#[macro_export]
macro_rules! track_memory {
    () => {};
}

#[cfg(feature = "editor")]
pub use engine_memory::TrackingAllocator;

pub struct GameConfig {
    pub title: String,
    pub window_size: (f64, f64),
    /// Fixed simulation rate (see [`Game::fixed_update`]).
    pub fixed_hz: f64,
    pub renderer: RendererConfig,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            title: "Game".to_owned(),
            window_size: (1280.0, 720.0),
            fixed_hz: 60.0,
            renderer: RendererConfig::default(),
        }
    }
}

/// The state a game owns and may freely change. Fields are public on purpose.
pub struct Engine {
    pub world: World,
    pub camera: FlyCamera,
    pub env: Environment,
    pub sim: SimControl,
    /// Set to `true` to close the game after the current frame.
    pub quit: bool,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self {
            world: World::new(),
            camera: FlyCamera::new(Vec3::new(0.0, 5.0, 20.0), 0.0, -0.2),
            env: Environment::default(),
            sim: SimControl::default(),
            quit: false,
        }
    }
}

/// Implement this for a game. All methods receive the full [`Engine`] state.
pub trait Game: 'static {
    /// Once, before the window opens: spawn entities, place the camera, tune the environment.
    /// The camera and environment as left here are what the editor's "Reset" restores.
    fn setup(&mut self, engine: &mut Engine);

    /// Gameplay and physics at a constant rate (`GameConfig::fixed_hz`), zero or more times per
    /// frame. Transforms changed here are interpolated for rendering automatically; the engine
    /// snapshots them before each call. Honours the editor's pause / step / time scale.
    fn fixed_update(&mut self, _engine: &mut Engine, _dt: f32) {}

    /// Once per rendered frame with the real frame time: camera, input, anything not simulation.
    fn update(&mut self, _engine: &mut Engine, _input: &engine_platform::Input, _dt: f32) {}
}

/// Opens the window and runs `game` until it quits.
pub fn run<G: Game>(config: GameConfig, game: G) -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).try_init();
    let window = engine_platform::WindowConfig {
        title: config.title.clone(),
        logical_size: config.window_size,
        fixed_hz: config.fixed_hz,
    };
    engine_platform::run(window, app::EngineApp::new(game, config))?;
    Ok(())
}
