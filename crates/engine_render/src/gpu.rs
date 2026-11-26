use std::fmt;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::config::RendererConfig;
use crate::targets::DEPTH_FORMAT;

#[derive(Debug)]
pub enum GpuError {
    Surface(wgpu::CreateSurfaceError),
    Device(wgpu::RequestDeviceError),
    NoSurfaceFormat,
    NoAdapter,
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(e) => write!(f, "surface creation failed: {e}"),
            Self::Device(e) => write!(f, "device request failed: {e}"),
            Self::NoSurfaceFormat => write!(f, "surface reports no supported formats"),
            Self::NoAdapter => write!(f, "no adapter can present to this surface"),
        }
    }
}

impl std::error::Error for GpuError {}

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter_info: wgpu::AdapterInfo,
    /// Effective MSAA sample count (may be lower than requested).
    pub sample_count: u32,
    pub vsync: bool,
    adapter: wgpu::Adapter,
}

impl GpuContext {
    pub fn new<W>(window: W, size: (u32, u32), settings: &RendererConfig) -> Result<Self, GpuError>
    where
        W: HasWindowHandle + HasDisplayHandle + Clone + std::fmt::Debug + Send + Sync + 'static,
    {
        // Honours WGPU_BACKEND (vulkan | gl | ...) and related env vars; the display handle is
        // required by the GL (EGL) backend.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
            window.clone(),
        )));
        let surface = instance.create_surface(window).map_err(GpuError::Surface)?;

        let adapter = select_adapter(&instance, &surface)?;
        log::info!("adapter: {:?}", adapter.get_info());

        // Downlevel baseline (GLES 3.0 class) raised to whatever the adapter really offers,
        // so older GPUs / the GL backend are not rejected for limits the engine never uses.
        let required_limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("engine device"),
            required_limits,
            ..Default::default()
        }))
        .map_err(GpuError::Device)?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(GpuError::NoSurfaceFormat)?;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: if settings.vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let sample_count = resolve_sample_count(&adapter, format, settings.msaa_samples);
        let adapter_info = adapter.get_info();
        Ok(Self { surface, device, queue, config, adapter_info, sample_count, vsync: settings.vsync, adapter })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        self.vsync = vsync;
        self.config.present_mode =
            if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        self.surface.configure(&self.device, &self.config);
    }

    /// Stores the effective count for `requested` (see `resolve_sample_count`) and returns it.
    /// The caller must rebuild everything that depends on the sample count.
    pub fn set_sample_count(&mut self, requested: u32) -> u32 {
        self.sample_count = resolve_sample_count(&self.adapter, self.config.format, requested);
        self.sample_count
    }

    /// Support flags for `SAMPLE_COUNTS` (1, 2, 4, 8), for both colour and depth.
    pub fn supported_sample_counts(&self) -> [bool; 4] {
        SAMPLE_COUNTS.map(|n| supports_samples(&self.adapter, self.config.format, n))
    }
}

/// MSAA sample counts the renderer can be asked for.
pub const SAMPLE_COUNTS: [u32; 4] = [1, 2, 4, 8];

fn supports_samples(adapter: &wgpu::Adapter, color: wgpu::TextureFormat, n: u32) -> bool {
    n == 1
        || [color, DEPTH_FORMAT]
            .into_iter()
            .all(|f| adapter.get_texture_format_features(f).flags.sample_count_supported(n))
}

/// Lower is better. A CPU rasteriser (llvmpipe) is only used when nothing else can present.
fn adapter_rank(ty: wgpu::DeviceType) -> u8 {
    match ty {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    }
}

/// Enumerates every backend the instance allows (Vulkan, GL, ...), keeps adapters that can
/// present to `surface`, and picks the best hardware one. Without this, a machine whose GPU has
/// no Vulkan driver silently ends up on the software Vulkan device.
fn select_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface<'_>) -> Result<wgpu::Adapter, GpuError> {
    let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    adapters.retain(|a| a.is_surface_supported(surface));
    for a in &adapters {
        let i = a.get_info();
        log::info!("candidate adapter: {} ({:?}, {:?})", i.name, i.device_type, i.backend);
    }
    adapters
        .into_iter()
        .min_by_key(|a| adapter_rank(a.get_info().device_type))
        .ok_or(GpuError::NoAdapter)
}

/// Largest supported sample count not above `requested` for both colour and depth, else 1.
fn resolve_sample_count(adapter: &wgpu::Adapter, color: wgpu::TextureFormat, requested: u32) -> u32 {
    let mut n = requested.max(1);
    while n > 1 && !supports_samples(adapter, color, n) {
        n /= 2;
    }
    if n != requested {
        log::warn!("MSAA x{requested} unsupported on this adapter/format; using x{n}");
    }
    n
}
