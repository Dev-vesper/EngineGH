use dear_imgui_rs::Ui;

pub(super) fn draw(ui: &Ui) {
    ui.text("Left click (scene)  capture mouse");
    ui.text("Esc                 release mouse / quit");
    ui.text("W A S D             move");
    ui.text("Space / Ctrl        up / down");
    ui.text("Shift               fast");
    ui.text("F1                  show / hide editor");
    ui.separator();
    ui.text_disabled("While the mouse is captured the UI ignores it.");
}
