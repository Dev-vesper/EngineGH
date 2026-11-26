//! Serialisable benchmark data model. Bump `SCHEMA_VERSION` on any breaking change.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ScenarioSpec {
    pub instances: u32,
    /// Requested MSAA samples; the effective value is in `Measurements`.
    pub msaa_samples: u32,
    pub vsync: bool,
    pub gui: bool,
}

/// All values in milliseconds.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Distribution {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryReport {
    /// Live Rust-heap bytes when the run ended.
    pub heap_end_bytes: usize,
    pub heap_peak_bytes: usize,
    pub alloc_count: usize,
    /// Linux only.
    pub rss_end_bytes: Option<usize>,
    pub rss_peak_bytes: Option<usize>,
    /// Live Rust-heap bytes at the start of the measured window (after warm-up).
    pub heap_start_bytes: usize,
    /// `heap_end - heap_start` over the measured window. Sustained growth here is a leak;
    /// 0 (or tiny, non-scaling noise) means steady state.
    pub heap_growth_bytes: i64,
    /// Rust-allocator calls per measured frame (includes wgpu/driver-side Rust allocations).
    pub allocs_per_frame: f64,
    /// Resident-set growth over the measured window (Linux only).
    pub rss_growth_bytes: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Measurements {
    pub effective_msaa_samples: u32,
    pub warmup_frames: u32,
    /// Number of measured frame intervals.
    pub frames: u32,
    /// Presented frames that were skipped (occluded, lost surface, ...) during the whole run.
    pub skipped_frames: u32,
    pub duration_s: f64,
    pub fps_avg: f64,
    /// Average FPS over the slowest 1% of frames.
    pub fps_1pct_low: f64,
    /// Time between the starts of consecutive frames (includes present / vsync wait).
    pub frame_interval_ms: Distribution,
    /// CPU time inside `Renderer::render` (acquire + encode + submit + present).
    pub render_call_ms: Distribution,
    pub memory: MemoryReport,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdapterReport {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub driver: String,
    pub driver_info: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EnvironmentReport {
    pub os: String,
    pub arch: String,
    /// "debug" or "release". Only release numbers are meaningful.
    pub build_profile: String,
    pub window_width: u32,
    pub window_height: u32,
    pub adapter: AdapterReport,
}

/// What one child process prints (single JSON line on stdout).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChildOutput {
    pub environment: EnvironmentReport,
    pub measurements: Measurements,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScenarioReport {
    pub scenario: ScenarioSpec,
    pub measurements: Option<Measurements>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Report {
    pub schema_version: u32,
    pub benchmark_version: String,
    pub timestamp_unix: u64,
    pub environment: Option<EnvironmentReport>,
    pub results: Vec<ScenarioReport>,
}
