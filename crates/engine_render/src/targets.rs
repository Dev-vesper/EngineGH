/// Number of MSAA samples for the main pass.
pub const SAMPLE_COUNT: u32 = 4;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Size-dependent attachments of the main pass. Recreated on resize.
pub struct RenderTargets {
    pub color_msaa: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}

impl RenderTargets {
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let make = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: SAMPLE_COUNT,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        Self { color_msaa: make("msaa color", color_format), depth: make("depth", DEPTH_FORMAT) }
    }
}
