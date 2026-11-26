use dear_imgui_rs::{Condition, TreeNodeFlags, Ui};
use engine_camera::FlyCamera;
use engine_render::Environment;
use glam::Vec3;

use engine_memory::MemoryStats;
use crate::stats::FrameStats;

/// Mutable engine state the inspector edits. Borrowed for one frame.
pub struct InspectorData<'a> {
    pub camera: &'a mut FlyCamera,
    pub env: &'a mut Environment,
    pub instance_count: usize,
}

pub(crate) struct PanelContext<'a> {
    pub stats: &'a FrameStats,
    pub memory: &'a MemoryStats,
    pub adapter: &'a str,
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn edit_vec3(ui: &Ui, label: &str, v: &mut Vec3) {
    let mut a = v.to_array();
    if ui.drag_float3(label, &mut a) {
        *v = Vec3::from_array(a);
    }
}

pub(crate) fn draw(ui: &Ui, cx: &PanelContext<'_>, data: &mut InspectorData<'_>) {
    ui.window("Engine")
        .size([340.0, 520.0], Condition::FirstUseEver)
        .position([12.0, 12.0], Condition::FirstUseEver)
        .build(|| {
            if ui.collapsing_header("Performance", TreeNodeFlags::DEFAULT_OPEN) {
                ui.text(format!("{:.0} FPS  ({:.2} ms)", cx.stats.fps(), cx.stats.frame_ms()));
                ui.plot_lines("##frametime", cx.stats.history_ms());
            }

            if ui.collapsing_header("Renderer", TreeNodeFlags::DEFAULT_OPEN) {
                ui.text(format!("Adapter: {}", cx.adapter));
                ui.text(format!("Instances: {}", data.instance_count));
            }

            if ui.collapsing_header("Memory", TreeNodeFlags::DEFAULT_OPEN) {
                let m = cx.memory;
                ui.text(format!("Rust heap:  {:.2} MiB", mib(m.heap_bytes)));
                ui.text(format!("Heap peak:  {:.2} MiB", mib(m.heap_peak_bytes)));
                ui.text(format!("Allocs/s:   {}", m.allocs_per_sec));
                match m.rss_bytes {
                    Some(rss) => {
                        ui.text(format!("Process RSS: {:.2} MiB", mib(rss)));
                        ui.text_disabled(format!(
                            "non-Rust (C++/driver/libs): {:.2} MiB",
                            mib(rss.saturating_sub(m.heap_bytes))
                        ));
                    }
                    None => ui.text_disabled("RSS unavailable on this OS"),
                }
            }

            if ui.collapsing_header("Camera", TreeNodeFlags::DEFAULT_OPEN) {
                edit_vec3(ui, "Position", &mut data.camera.position);
                ui.slider_f32("Move speed", &mut data.camera.move_speed, 0.5, 50.0);
                ui.slider_f32("Look sens.", &mut data.camera.look_sensitivity, 0.0005, 0.01);
                let mut fov = data.camera.projection.fov_y.to_degrees();
                if ui.slider_f32("FOV", &mut fov, 30.0, 120.0) {
                    data.camera.projection.fov_y = fov.to_radians();
                }
            }

            if ui.collapsing_header("Environment", TreeNodeFlags::DEFAULT_OPEN) {
                edit_vec3(ui, "Light dir", &mut data.env.light_dir);
                let mut sky = data.env.sky.to_array();
                if ui.color_edit3("Sky / fog", &mut sky) {
                    data.env.sky = Vec3::from_array(sky);
                }
                ui.slider_f32("Fog density", &mut data.env.fog_density, 0.0, 0.1);
            }
        });
}
