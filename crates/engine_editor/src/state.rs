use engine_camera::FlyCamera;
use engine_render::{Environment, Instance, RenderStatus};
use engine_time::SimControl;
use glam::{Mat4, Quat, Vec4};

/// Engine state the editor may read or edit directly during a frame. Everything the renderer
/// holds borrowed while the UI runs (scene instances, renderer settings) is read-only here and
/// changed through [`EditorOutput`] instead.
pub struct InspectorData<'a> {
    pub camera: &'a mut FlyCamera,
    pub env: &'a mut Environment,
    pub sim: &'a mut SimControl,
    pub instances: &'a [Instance],
    pub render: RenderStatus,
}

/// One deferred scene edit; the application applies it after the frame.
#[derive(Clone, Copy, Debug, Default)]
pub enum SceneOp {
    #[default]
    None,
    Add { model: Mat4, color: Vec4 },
    Duplicate(usize),
    Remove(usize),
    Set { index: usize, model: Mat4, color: Vec4 },
}

/// Requests produced by the UI during one frame. Apply them after rendering.
#[derive(Debug, Default)]
pub struct EditorOutput {
    pub vsync: Option<bool>,
    /// Requested MSAA samples (effective value may differ, see `RenderStatus`).
    pub msaa: Option<u32>,
    pub reset_camera: bool,
    pub reset_environment: bool,
    pub quit: bool,
    /// At most one scene edit per frame.
    pub scene: SceneOp,
}

pub(crate) struct PanelVisibility {
    pub performance: bool,
    pub renderer: bool,
    pub memory: bool,
    pub simulation: bool,
    pub camera: bool,
    pub environment: bool,
    pub scene: bool,
    pub help: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            performance: true,
            renderer: true,
            memory: true,
            simulation: true,
            camera: true,
            environment: true,
            scene: true,
            help: false,
        }
    }
}

/// Which instance the scene panel edits, plus its rotation as stable Euler angles
/// (re-deriving Euler angles from the matrix every frame would make them jump near gimbal lock).
pub(crate) struct SceneSelection {
    pub index: usize,
    pub euler_deg: [f32; 3],
    pub rotation: Quat,
    pub cached_for: Option<usize>,
}

impl Default for SceneSelection {
    fn default() -> Self {
        Self { index: 0, euler_deg: [0.0; 3], rotation: Quat::IDENTITY, cached_for: None }
    }
}

#[derive(Default)]
pub(crate) struct EditorState {
    pub panels: PanelVisibility,
    pub selection: SceneSelection,
}
