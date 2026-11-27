use engine_camera::FlyCamera;
use engine_platform::{Input, Key};
use glam::Vec3;

/// Default free-fly controls: mouse look while the cursor is captured, WASD, Space / Ctrl for
/// up / down, Shift for 3x speed. Call from `Game::update`, or write your own.
pub fn fly_camera(camera: &mut FlyCamera, input: &Input, dt: f32) {
    if input.cursor_grabbed() {
        camera.look(input.mouse_delta());
    }
    let axes = Vec3::new(
        input.axis(Key::KeyD, Key::KeyA),
        input.axis(Key::Space, Key::ControlLeft),
        input.axis(Key::KeyW, Key::KeyS),
    );
    let boost = if input.is_down(Key::ShiftLeft) { 3.0 } else { 1.0 };
    camera.translate(axes, boost, dt);
}
