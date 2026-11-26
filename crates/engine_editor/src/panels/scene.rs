use dear_imgui_rs::Ui;
use engine_camera::FlyCamera;
use engine_render::Instance;
use glam::{EulerRot, Mat4, Quat, Vec3, Vec4};

use super::drag_vec3;
use crate::state::{EditorOutput, SceneOp, SceneSelection};

const EULER: EulerRot = EulerRot::YXZ;
const MIN_SCALE: f32 = 0.001;
/// Distance in front of the camera where new objects appear.
const SPAWN_DISTANCE: f32 = 6.0;

fn euler_to_quat(deg: [f32; 3]) -> Quat {
    Quat::from_euler(EULER, deg[1].to_radians(), deg[0].to_radians(), deg[2].to_radians())
}

pub(super) fn draw(
    ui: &Ui,
    sel: &mut SceneSelection,
    instances: &[Instance],
    camera: &FlyCamera,
    out: &mut EditorOutput,
) {
    ui.text(format!("Instances: {}", instances.len()));

    if ui.button("Add cube") {
        out.scene = SceneOp::Add {
            model: Mat4::from_translation(camera.position + camera.forward() * SPAWN_DISTANCE),
            color: Vec4::new(0.8, 0.8, 0.8, 1.0),
        };
    }
    if instances.is_empty() {
        return;
    }

    sel.index = sel.index.min(instances.len() - 1);
    let mut index = sel.index as i32;
    if ui.slider_i32("Index", &mut index, 0, instances.len() as i32 - 1) {
        sel.index = index as usize;
    }
    let instance = instances[sel.index];
    let (mut scale, rotation, mut position) = instance.model.to_scale_rotation_translation();

    // Re-derive the Euler angles only when the selection or the object changed under us.
    if sel.cached_for != Some(sel.index) || rotation.dot(sel.rotation).abs() < 0.9999 {
        let (y, x, z) = rotation.to_euler(EULER);
        sel.euler_deg = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
        sel.rotation = rotation;
        sel.cached_for = Some(sel.index);
    }

    let mut changed = drag_vec3(ui, "Position", &mut position, 0.05);
    let mut euler = Vec3::from_array(sel.euler_deg);
    if drag_vec3(ui, "Rotation", &mut euler, 0.5) {
        sel.euler_deg = euler.to_array();
        sel.rotation = euler_to_quat(sel.euler_deg);
        changed = true;
    }
    changed |= drag_vec3(ui, "Scale", &mut scale, 0.02);
    let mut rgb = instance.color.truncate().to_array();
    changed |= ui.color_edit3("Color", &mut rgb);

    if changed {
        out.scene = SceneOp::Set {
            index: sel.index,
            model: Mat4::from_scale_rotation_translation(scale.max(Vec3::splat(MIN_SCALE)), sel.rotation, position),
            color: Vec4::new(rgb[0], rgb[1], rgb[2], instance.color.w),
        };
    }

    ui.separator();
    if ui.button("Duplicate") {
        out.scene = SceneOp::Duplicate(sel.index);
    }
    ui.same_line();
    if ui.button("Remove") {
        out.scene = SceneOp::Remove(sel.index);
    }
}
