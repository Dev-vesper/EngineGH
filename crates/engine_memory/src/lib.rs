//! Heap accounting shared by the editor and the benchmark.
//!
//! `TrackingAllocator` measures every Rust-side allocation. Memory allocated by C/C++ code
//! (Dear ImGui, GPU drivers) bypasses it; compare with the process RSS to see that remainder.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Install with `#[global_allocator] static A: TrackingAllocator = TrackingAllocator;`
pub struct TrackingAllocator;

#[inline]
fn grow(bytes: usize) {
    let now = CURRENT.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(now, Ordering::Relaxed);
}

// SAFETY: every method forwards to `System` with the caller's unchanged arguments, so all
// `GlobalAlloc` contracts are upheld by `System`; the atomics only observe sizes and never
// touch the memory or allocate.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            grow(layout.size());
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc_zeroed(layout);
        if !ptr.is_null() {
            grow(layout.size());
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                grow(new_size - layout.size());
            } else {
                CURRENT.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[derive(Clone, Copy, Default)]
pub struct MemoryStats {
    /// Live bytes allocated through the Rust global allocator.
    pub heap_bytes: usize,
    pub heap_peak_bytes: usize,
    /// Total allocation calls since start.
    pub alloc_count: usize,
    /// Allocation calls during the previous full second (0 until one has elapsed).
    pub allocs_per_sec: usize,
    /// Process resident set size (Linux only).
    pub rss_bytes: Option<usize>,
}

/// Samples counters about once a second so the panel itself costs (almost) nothing per frame.
pub struct MemorySampler {
    last: Instant,
    last_alloc_count: usize,
    stats: MemoryStats,
}

impl MemoryStats {
    /// Reads all counters right now (`allocs_per_sec` is 0; use `MemorySampler` for rates).
    pub fn now() -> Self {
        Self {
            heap_bytes: CURRENT.load(Ordering::Relaxed),
            heap_peak_bytes: PEAK.load(Ordering::Relaxed),
            alloc_count: ALLOC_COUNT.load(Ordering::Relaxed),
            allocs_per_sec: 0,
            rss_bytes: resident_bytes(),
        }
    }
}

impl MemorySampler {
    const PERIOD: Duration = Duration::from_secs(1);

    pub fn new() -> Self {
        let mut s = Self { last: Instant::now(), last_alloc_count: 0, stats: MemoryStats::default() };
        s.sample(Duration::from_secs(1));
        s
    }

    pub fn update(&mut self) -> &MemoryStats {
        let elapsed = self.last.elapsed();
        if elapsed >= Self::PERIOD {
            self.sample(elapsed);
        }
        &self.stats
    }

    fn sample(&mut self, elapsed: Duration) {
        let mut stats = MemoryStats::now();
        stats.allocs_per_sec = ((stats.alloc_count - self.last_alloc_count) as f64 / elapsed.as_secs_f64()) as usize;
        self.last_alloc_count = stats.alloc_count;
        self.stats = stats;
        self.last = Instant::now();
    }
}

impl Default for MemorySampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
fn proc_status_bytes(key: &str) -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with(key))?;
    let kb: usize = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[cfg(target_os = "linux")]
fn resident_bytes() -> Option<usize> {
    proc_status_bytes("VmRSS:")
}

/// Peak resident set size of the process so far (Linux only).
#[cfg(target_os = "linux")]
pub fn peak_resident_bytes() -> Option<usize> {
    proc_status_bytes("VmHWM:")
}

#[cfg(not(target_os = "linux"))]
fn resident_bytes() -> Option<usize> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn peak_resident_bytes() -> Option<usize> {
    None
}
