use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

use crate::gpu::{GpuContext, GpuError};
use crate::grid::{build_grid, LineVertex};
use crate::instance::{DynamicBuffer, Instance};
use crate::mesh::{unit_cube, GpuMesh, MeshVertex};
use crate::targets::{RenderTargets, DEPTH_FORMAT, SAMPLE_COUNT};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: Mat4,
    eye: Vec4,
    light_dir: Vec4,
    fog: Vec4,
}

/// Everything the renderer needs for one frame.
pub struct Frame<'a> {
    pub view_proj: Mat4,
    pub eye: Vec3,
    /// Direction TOWARDS the light (need not be normalised).
    pub light_dir: Vec3,
    /// Linear RGB. Used as clear colour and fog colour.
    pub sky: Vec3,
    pub fog_density: f32,
    pub instances: &'a [Instance],
}

pub struct Renderer {
    gpu: GpuContext,
    targets: RenderTargets,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    mesh_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    cube: GpuMesh,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    instances: DynamicBuffer,
}

impl Renderer {
    pub fn new<W>(window: W, size: (u32, u32)) -> Result<Self, GpuError>
    where
        W: HasWindowHandle + HasDisplayHandle + Clone + std::fmt::Debug + Send + Sync + 'static,
    {
        let gpu = GpuContext::new(window, size)?;
        let device = &gpu.device;
        let format = gpu.config.format;

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });

        let make_pipeline = |label: &str,
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
                multisample: wgpu::MultisampleState { count: SAMPLE_COUNT, mask: !0, alpha_to_coverage_enabled: false },
                multiview_mask: None,
                cache: None,
            })
        };
        let mesh_pipeline = make_pipeline(
            "mesh pipeline",
            "vs_mesh",
            "fs_mesh",
            &[Some(MeshVertex::LAYOUT), Some(Instance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            Some(wgpu::Face::Back),
        );
        let line_pipeline = make_pipeline(
            "line pipeline",
            "vs_line",
            "fs_line",
            &[Some(LineVertex::LAYOUT)],
            wgpu::PrimitiveTopology::LineList,
            None,
        );

        let cube = GpuMesh::upload(device, "unit cube", &unit_cube());
        let grid = build_grid(100, 10);
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid"),
            contents: bytemuck::cast_slice(&grid),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let (w, h) = gpu.size();
        let targets = RenderTargets::new(device, format, w, h);
        let instances = DynamicBuffer::new(device, "instances", wgpu::BufferUsages::VERTEX, 64 * 1024);

        Ok(Self {
            gpu,
            targets,
            camera_buffer,
            camera_bind_group,
            mesh_pipeline,
            line_pipeline,
            cube,
            grid_buffer,
            grid_vertex_count: grid.len() as u32,
            instances,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.gpu.resize(width, height);
        self.targets = RenderTargets::new(&self.gpu.device, self.gpu.config.format, width, height);
    }

    #[inline]
    pub fn aspect(&self) -> f32 {
        let (w, h) = self.gpu.size();
        w as f32 / h as f32
    }

    pub fn render(&mut self, frame: &Frame<'_>) {
        let surface_texture = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let (w, h) = self.gpu.size();
                self.gpu.surface.configure(&self.gpu.device, &self.gpu.config);
                self.targets = RenderTargets::new(&self.gpu.device, self.gpu.config.format, w, h);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("surface validation error while acquiring frame");
                return;
            }
        };
        let resolve_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let light = frame.light_dir.normalize_or_zero();
        let uniform = CameraUniform {
            view_proj: frame.view_proj,
            eye: frame.eye.extend(1.0),
            light_dir: light.extend(0.0),
            fog: frame.sky.extend(frame.fog_density),
        };
        self.gpu.queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));
        self.instances.write(&self.gpu.device, &self.gpu.queue, bytemuck::cast_slice(frame.instances));

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.color_msaa,
                    depth_slice: None,
                    resolve_target: Some(&resolve_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: frame.sky.x as f64,
                            g: frame.sky.y as f64,
                            b: frame.sky.z as f64,
                            a: 1.0,
                        }),
                        // MSAA contents are only needed for the resolve.
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0), // reverse-Z
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            if !frame.instances.is_empty() {
                pass.set_pipeline(&self.mesh_pipeline);
                pass.set_vertex_buffer(0, self.cube.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.instances.buffer().slice(..));
                pass.set_index_buffer(self.cube.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..self.cube.index_count, 0, 0..frame.instances.len() as u32);
            }

            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.grid_buffer.slice(..));
            pass.draw(0..self.grid_vertex_count, 0..1);
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(surface_texture);
    }
}
