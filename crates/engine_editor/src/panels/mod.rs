//! One module per panel. Each `draw` is a plain function over the data it needs.

mod camera;
mod environment;
mod help;
mod memory;
mod performance;
mod renderer;
mod scene;
mod simulation;

use dear_imgui_rs::{Condition, Ui};
use engine_memory::MemoryStats;
use glam::Vec3;

use crate::state::{EditorOutput, EditorState, InspectorData};
use crate::stats::FrameStats;

const PANEL_WIDTH: f32 = 310.0;
const MARGIN: f32 = 8.0;
const MENU_BAR_HEIGHT: f32 = 22.0;

/// Read-only per-frame inputs shared by the panels.
pub(crate) struct PanelContext<'a> {
    pub stats: &'a FrameStats,
    pub memory: &'a MemoryStats,
    pub adapter: &'a str,
}

/// A titled, closable, auto-height window placed once on first use.
fn panel(ui: &Ui, title: &str, open: &mut bool, pos: [f32; 2], body: impl FnOnce()) {
    if !*open {
        return;
    }
    ui.window(title)
        .opened(open)
        .position(pos, Condition::FirstUseEver)
        .size([PANEL_WIDTH, 0.0], Condition::FirstUseEver)
        .build(body);
}

/// Three drag fields on one row followed by a label. `speed` is value change per pixel.
fn drag_vec3(ui: &Ui, label: &str, value: &mut Vec3, speed: f32) -> bool {
    const IDS: [&str; 3] = ["##x", "##y", "##z"];
    let _id = ui.push_id(label);
    let width = ((ui.content_region_avail_width() - 90.0) / 3.0).max(40.0);
    let mut components = value.to_array();
    let mut changed = false;
    for (i, c) in components.iter_mut().enumerate() {
        if i > 0 {
            ui.same_line();
        }
        ui.set_next_item_width(width);
        changed |= ui.drag_float_config(IDS[i]).speed(speed).build(ui, c);
    }
    ui.same_line();
    ui.text(label);
    if changed {
        *value = Vec3::from_array(components);
    }
    changed
}

pub(crate) fn draw_all(
    ui: &Ui,
    state: &mut EditorState,
    cx: &PanelContext<'_>,
    data: &mut InspectorData<'_>,
    out: &mut EditorOutput,
) {
    draw_menu_bar(ui, state, cx, out);

    let display_w = ui.io().display_size()[0];
    let left = MARGIN;
    let right = (display_w - PANEL_WIDTH - MARGIN).max(left);
    let top = MENU_BAR_HEIGHT + MARGIN;
    let p = &mut state.panels;

    panel(ui, "Performance", &mut p.performance, [left, top], || performance::draw(ui, cx.stats));
    panel(ui, "Renderer", &mut p.renderer, [left, top + 130.0], || {
        renderer::draw(ui, cx.adapter, &data.render, data.instances.len(), out)
    });
    panel(ui, "Memory", &mut p.memory, [left, top + 300.0], || memory::draw(ui, cx.memory));
    panel(ui, "Simulation", &mut p.simulation, [left, top + 455.0], || simulation::draw(ui, data.sim));
    panel(ui, "Camera", &mut p.camera, [right, top], || camera::draw(ui, data.camera, out));
    panel(ui, "Environment", &mut p.environment, [right, top + 250.0], || {
        environment::draw(ui, data.env, out)
    });
    panel(ui, "Scene", &mut p.scene, [right, top + 395.0], || {
        scene::draw(ui, &mut state.selection, data.instances, data.camera, out)
    });
    panel(ui, "Controls", &mut state.panels.help, [left, top], || help::draw(ui));
}

fn draw_menu_bar(ui: &Ui, state: &mut EditorState, cx: &PanelContext<'_>, out: &mut EditorOutput) {
    ui.main_menu_bar(|| {
        if let Some(_menu) = ui.begin_menu("Engine") {
            if ui.menu_item("Reset camera") {
                out.reset_camera = true;
            }
            if ui.menu_item("Quit") {
                out.quit = true;
            }
        }
        if let Some(_menu) = ui.begin_menu("View") {
            let p = &mut state.panels;
            ui.menu_item_toggle_no_shortcut("Performance", &mut p.performance, true);
            ui.menu_item_toggle_no_shortcut("Renderer", &mut p.renderer, true);
            ui.menu_item_toggle_no_shortcut("Memory", &mut p.memory, true);
            ui.menu_item_toggle_no_shortcut("Simulation", &mut p.simulation, true);
            ui.menu_item_toggle_no_shortcut("Camera", &mut p.camera, true);
            ui.menu_item_toggle_no_shortcut("Environment", &mut p.environment, true);
            ui.menu_item_toggle_no_shortcut("Scene", &mut p.scene, true);
            ui.menu_item_toggle_no_shortcut("Controls", &mut p.help, true);
        }
        ui.text_disabled(format!("  {:.0} FPS", cx.stats.fps()));
    });
}
