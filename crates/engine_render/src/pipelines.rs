use crate::grid::LineVertex;
use crate::instance::Instance;
use crate::mesh::MeshVertex;
use crate::targets::DEPTH_FORMAT;

/// Render pipelines of the main pass. Rebuilt when the MSAA sample count changes.
pub struct Pipelines {
    pub mesh: wgpu::RenderPipeline,
    pub line: wgpu::RenderPipeline,
}

impl Pipelines {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene layout"),
            bind_group_layouts: &[Some(camera_layout)],
            immediate_size: 0,
        });

        let make = |label: &str,
                    vs: &str,
                    fs: &str,
                    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                    topology: wgpu::PrimitiveTopology,
                    cull: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    ..Default::default()
                },
                // Reverse-Z: near = 1, far = 0.
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: sample_count, mask: !0, alpha_to_coverage_enabled: false },
                multiview_mask: None,
                cache: None,
            })
        };

        Self {
            mesh: make(
                "mesh pipeline",
                "vs_mesh",
                "fs_mesh",
                &[Some(MeshVertex::LAYOUT), Some(Instance::LAYOUT)],
                wgpu::PrimitiveTopology::TriangleList,
                Some(wgpu::Face::Back),
            ),
            line: make(
                "line pipeline",
                "vs_line",
                "fs_line",
                &[Some(LineVertex::LAYOUT)],
                wgpu::PrimitiveTopology::LineList,
                None,
            ),
        }
    }
}
