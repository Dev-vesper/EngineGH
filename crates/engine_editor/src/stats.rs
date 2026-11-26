/// Number of frame-time samples kept for the graph.
pub const HISTORY: usize = 120;

/// Frame timing with a fixed-size history (no allocation after construction).
pub struct FrameStats {
    ms: [f32; HISTORY],
    smoothed_ms: f32,
}

impl FrameStats {
    pub fn new() -> Self {
        Self { ms: [0.0; HISTORY], smoothed_ms: 0.0 }
    }

    pub fn push(&mut self, dt_seconds: f32) {
        let ms = dt_seconds * 1000.0;
        self.ms.copy_within(1.., 0);
        self.ms[HISTORY - 1] = ms;
        self.smoothed_ms = if self.smoothed_ms == 0.0 { ms } else { self.smoothed_ms * 0.95 + ms * 0.05 };
    }

    #[inline]
    pub fn history_ms(&self) -> &[f32] {
        &self.ms
    }

    #[inline]
    pub fn frame_ms(&self) -> f32 {
        self.smoothed_ms
    }

    #[inline]
    pub fn fps(&self) -> f32 {
        if self.smoothed_ms > 0.0 { 1000.0 / self.smoothed_ms } else { 0.0 }
    }
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}
