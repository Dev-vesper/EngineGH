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
    /// Selected object as an index into `instances` for this frame.
    pub selected: Option<usize>,
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

/// Change of selection requested by the UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectRequest {
    #[default]
    Keep,
    Clear,
    /// Index into this frame's `instances`.
    Index(usize),
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
    pub select: SelectRequest,
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

/// Rotation of the selected object as stable Euler angles (re-deriving them from the matrix
/// every frame would make them jump near gimbal lock).
pub(crate) struct SceneSelection {
    pub euler_deg: [f32; 3],
    pub rotation: Quat,
    pub cached_for: Option<usize>,
}

impl Default for SceneSelection {
    fn default() -> Self {
        Self { euler_deg: [0.0; 3], rotation: Quat::IDENTITY, cached_for: None }
    }
}

#[derive(Default)]
pub(crate) struct EditorState {
    pub panels: PanelVisibility,
    pub selection: SceneSelection,
}
