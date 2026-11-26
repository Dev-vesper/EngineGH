/// Renderer options fixed at creation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererConfig {
    /// Wait for vertical blank. `false` picks the lowest-latency mode the surface offers.
    pub vsync: bool,
    /// Requested MSAA sample count (1 disables MSAA). Falls back to 1 if the GPU lacks support.
    pub msaa_samples: u32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self { vsync: true, msaa_samples: 4 }
    }
}
