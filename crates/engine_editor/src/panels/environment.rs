use dear_imgui_rs::Ui;
use engine_render::Environment;
use glam::Vec3;

use super::drag_vec3;
use crate::state::EditorOutput;

pub(super) fn draw(ui: &Ui, env: &mut Environment, out: &mut EditorOutput) {
    drag_vec3(ui, "Light dir", &mut env.light_dir, 0.01);
    let mut sky = env.sky.to_array();
    if ui.color_edit3("Sky / fog", &mut sky) {
        env.sky = Vec3::from_array(sky);
    }
    ui.slider_f32("Fog density", &mut env.fog_density, 0.0, 0.1);
    if ui.button("Reset environment") {
        out.reset_environment = true;
    }
}
