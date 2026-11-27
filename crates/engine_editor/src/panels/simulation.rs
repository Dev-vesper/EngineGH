use dear_imgui_rs::Ui;
use engine_time::SimControl;

const MIN_SCALE: f32 = 0.05;
const MAX_SCALE: f32 = 4.0;

pub(super) fn draw(ui: &Ui, sim: &mut SimControl) {
    ui.checkbox("Paused", &mut sim.paused);
    ui.same_line();
    if ui.button("Step") {
        sim.request_step();
    }
    ui.slider_f32("Time scale", &mut sim.time_scale, MIN_SCALE, MAX_SCALE);
    if ui.button("1x") {
        sim.time_scale = 1.0;
    }
}
