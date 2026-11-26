use dear_imgui_rs::Ui;
use engine_render::{RenderStatus, SAMPLE_COUNTS};

use crate::state::EditorOutput;

const SAMPLE_LABELS: [&str; 4] = ["Off (x1)", "x2", "x4", "x8"];

pub(super) fn draw(ui: &Ui, adapter: &str, status: &RenderStatus, instances: usize, out: &mut EditorOutput) {
    ui.text_wrapped(format!("Adapter: {adapter}"));
    ui.text(format!("Surface: {}x{}", status.size.0, status.size.1));
    ui.text(format!("Instances: {instances}"));
    ui.separator();

    let mut vsync = status.vsync;
    if ui.checkbox("VSync", &mut vsync) {
        out.vsync = Some(vsync);
    }
    ui.text_disabled("(the compositor/driver may force it on)");

    // Only offer sample counts this adapter supports.
    let mut labels = [""; 4];
    let mut values = [0u32; 4];
    let mut count = 0;
    for (i, &supported) in status.supported_sample_counts.iter().enumerate() {
        if supported {
            labels[count] = SAMPLE_LABELS[i];
            values[count] = SAMPLE_COUNTS[i];
            count += 1;
        }
    }
    let mut current = values[..count].iter().position(|&v| v == status.sample_count).unwrap_or(0);
    if ui.combo_simple_string("MSAA", &mut current, &labels[..count]) {
        out.msaa = Some(values[current]);
    }
}
