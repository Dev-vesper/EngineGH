//! Sandbox game: the template for new games. Copy this folder to `games/<name>`, rename the
//! package in its `Cargo.toml`, and run `cargo run -p <name> --release`.

use engine_app::prelude::*;

engine_app::track_memory!();

/// Angular velocity around world Y in rad/s. A component is just a struct.
struct Spin(f32);

struct SandboxGame;

impl Game for SandboxGame {
    fn setup(&mut self, engine: &mut Engine) {
        populate(&mut engine.world);
    }

    fn fixed_update(&mut self, engine: &mut Engine, dt: f32) {
        spin_system(&mut engine.world, dt);
    }

    fn update(&mut self, engine: &mut Engine, input: &Input, dt: f32) {
        fly_camera(&mut engine.camera, input, dt);
    }
}

fn cube(world: &mut World, position: Vec3, scale: Vec3, rotation: Quat, rgb: [f32; 3]) -> Entity {
    spawn_cube(
        world,
        Transform { position, rotation, scale },
        RenderCube { color: Vec4::new(rgb[0], rgb[1], rgb[2], 1.0) },
    )
}

fn populate(world: &mut World) {
    // Ground slab: top face at y = -0.002 so it never z-fights with the y = 0 grid.
    cube(world, Vec3::new(0.0, -0.052, 0.0), Vec3::new(400.0, 0.1, 400.0), Quat::IDENTITY, [0.015, 0.018, 0.022]);

    // Ring of spinning cubes of varying size and tilt.
    const COUNT: usize = 12;
    for i in 0..COUNT {
        let t = i as f32 / COUNT as f32;
        let angle = t * std::f32::consts::TAU;
        let size = 0.8 + 1.4 * (0.5 + 0.5 * (angle * 3.0).sin());
        let rot = Quat::from_rotation_y(angle) * Quat::from_rotation_x(0.35 * (angle * 2.0).cos());
        let pos = Vec3::new(angle.cos() * 8.0, size * 0.5 + 0.01, angle.sin() * 8.0);
        let rgb = [0.25 + 0.6 * t, 0.35 + 0.4 * (1.0 - t), 0.8 - 0.5 * t];
        let e = cube(world, pos, Vec3::splat(size), rot, rgb);
        let direction = if i % 2 == 0 { 1.0 } else { -1.0 };
        world.insert_one(e, Spin(0.6 * direction)).expect("entity was just spawned");
    }

    // Central tower (non-uniform scale exercises the normal matrix).
    cube(world, Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.5, 4.0, 1.5), Quat::IDENTITY, [0.7, 0.55, 0.2]);
}

/// A simulation system: a plain function over the world, called from `fixed_update`.
fn spin_system(world: &mut World, dt: f32) {
    for (transform, spin) in world.query_mut::<(&mut Transform, &Spin)>() {
        transform.rotation = (Quat::from_rotation_y(spin.0 * dt) * transform.rotation).normalize();
    }
}

fn main() {
    let config = GameConfig { title: "Engine Sandbox".to_owned(), ..Default::default() };
    if let Err(e) = engine_app::run(config, SandboxGame) {
        eprintln!("error: {e}");
    }
}
