use dear_imgui_rs::Ui;

use crate::stats::FrameStats;

pub(super) fn draw(ui: &Ui, stats: &FrameStats) {
    ui.text(format!("{:.0} FPS  ({:.2} ms)", stats.fps(), stats.frame_ms()));
    ui.plot_lines("##frametime", stats.history_ms());
}
