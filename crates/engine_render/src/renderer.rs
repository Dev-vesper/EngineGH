use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

use crate::gpu::{GpuContext, GpuError};
use crate::grid::{build_grid, LineVertex};
use crate::instance::{DynamicBuffer, Instance};
use crate::mesh::{unit_cube, GpuMesh, MeshVertex};
use crate::config::RendererConfig;
use crate::targets::{RenderTargets, DEPTH_FORMAT};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: Mat4,
    eye: Vec4,
    light_dir: Vec4,
    fog: Vec4,
}

/// Scene-wide lighting and atmosphere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Environment {
    /// Direction TOWARDS the light (need not be normalised).
    pub light_dir: Vec3,
    /// Linear RGB. Used as clear colour and fog colour.
    pub sky: Vec3,
    pub fog_density: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self { light_dir: Vec3::new(0.4, 0.9, 0.3), sky: Vec3::new(0.02, 0.04, 0.09), fog_density: 0.012 }
    }
}

/// Everything the renderer needs for one frame.
pub struct Frame<'a> {
    pub view_proj: Mat4,
    pub eye: Vec3,
    pub env: Environment,
    pub instances: &'a [Instance],
}

/// Records extra draw commands (e.g. a GUI) into a pass over the final, resolved surface image.
/// Receives the pass and the physical size of the surface texture.
pub type OverlayFn<'f> = &'f mut dyn FnMut(&mut wgpu::RenderPass<'_>, (u32, u32));

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
    pub fn new<W>(window: W, size: (u32, u32), config: RendererConfig) -> Result<Self, GpuError>
    where
        W: HasWindowHandle + HasDisplayHandle + Clone + std::fmt::Debug + Send + Sync + 'static,
    {
        let gpu = GpuContext::new(window, size, &config)?;
        let device = &gpu.device;
        let format = gpu.config.format;
        let sample_count = gpu.sample_count;

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
                multisample: wgpu::MultisampleState { count: sample_count, mask: !0, alpha_to_coverage_enabled: false },
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
        let targets = RenderTargets::new(device, format, w, h, sample_count);
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
        self.targets =
            RenderTargets::new(&self.gpu.device, self.gpu.config.format, width, height, self.gpu.sample_count);
    }

    /// Effective MSAA sample count.
    #[inline]
    pub fn sample_count(&self) -> u32 {
        self.gpu.sample_count
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.gpu.size()
    }

    #[inline]
    pub fn aspect(&self) -> f32 {
        let (w, h) = self.gpu.size();
        w as f32 / h as f32
    }

    #[inline]
    pub fn device(&self) -> &wgpu::Device {
        &self.gpu.device
    }

    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.gpu.queue
    }

    #[inline]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.gpu.config.format
    }

    #[inline]
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.gpu.adapter_info
    }

    /// Returns `true` if a frame was submitted and presented, `false` if it was skipped
    /// (surface lost/outdated/occluded/timeout).
    pub fn render(&mut self, frame: &Frame<'_>) -> bool {
        self.render_with_overlay(frame, None)
    }

    /// Like [`Renderer::render`], then runs `overlay` in a second pass (no depth, 1 sample)
    /// that loads the already-resolved surface image.
    pub fn render_with_overlay(&mut self, frame: &Frame<'_>, overlay: Option<OverlayFn<'_>>) -> bool {
        let surface_texture = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                let (w, h) = self.gpu.size();
                self.gpu.surface.configure(&self.gpu.device, &self.gpu.config);
                self.targets =
                    RenderTargets::new(&self.gpu.device, self.gpu.config.format, w, h, self.gpu.sample_count);
                return false;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return false,
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("surface validation error while acquiring frame");
                return false;
            }
        };
        let resolve_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let env = &frame.env;
        let uniform = CameraUniform {
            view_proj: frame.view_proj,
            eye: frame.eye.extend(1.0),
            light_dir: env.light_dir.normalize_or_zero().extend(0.0),
            fog: env.sky.extend(env.fog_density),
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
                    view: self.targets.color_msaa.as_ref().unwrap_or(&resolve_view),
                    depth_slice: None,
                    resolve_target: self.targets.color_msaa.as_ref().map(|_| &resolve_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: env.sky.x as f64,
                            g: env.sky.y as f64,
                            b: env.sky.z as f64,
                            a: 1.0,
                        }),
                        // MSAA contents are only needed for the resolve.
                        store: if self.targets.color_msaa.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
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
        if let Some(overlay) = overlay {
            let extent = (surface_texture.texture.width(), surface_texture.texture.height());
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("overlay pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &resolve_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            overlay(&mut pass, extent);
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        self.gpu.queue.present(surface_texture);
        true
    }
}
