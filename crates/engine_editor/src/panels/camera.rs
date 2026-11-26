use dear_imgui_rs::Ui;
use engine_camera::FlyCamera;

use super::drag_vec3;
use crate::state::EditorOutput;

pub(super) fn draw(ui: &Ui, camera: &mut FlyCamera, out: &mut EditorOutput) {
    drag_vec3(ui, "Position", &mut camera.position, 0.05);

    let mut yaw = camera.yaw.to_degrees();
    if ui.slider_f32("Yaw", &mut yaw, -180.0, 180.0) {
        camera.yaw = yaw.to_radians();
    }
    let mut pitch = camera.pitch.to_degrees();
    if ui.slider_f32("Pitch", &mut pitch, -89.0, 89.0) {
        camera.pitch = pitch.to_radians();
    }

    ui.separator();
    ui.slider_f32("Move speed", &mut camera.move_speed, 0.5, 100.0);
    ui.slider_f32("Look sens.", &mut camera.look_sensitivity, 0.0005, 0.01);
    let mut fov = camera.projection.fov_y.to_degrees();
    if ui.slider_f32("FOV", &mut fov, 30.0, 120.0) {
        camera.projection.fov_y = fov.to_radians();
    }
    ui.slider_f32("Near plane", &mut camera.projection.near, 0.01, 2.0);

    if ui.button("Reset camera") {
        out.reset_camera = true;
    }
}
