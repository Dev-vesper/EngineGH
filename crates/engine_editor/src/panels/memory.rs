use dear_imgui_rs::Ui;
use engine_memory::MemoryStats;

fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

pub(super) fn draw(ui: &Ui, m: &MemoryStats) {
    ui.text(format!("Rust heap:  {:.2} MiB", mib(m.heap_bytes)));
    ui.text(format!("Heap peak:  {:.2} MiB", mib(m.heap_peak_bytes)));
    ui.text(format!("Allocs/s:   {}", m.allocs_per_sec));
    match m.rss_bytes {
        Some(rss) => {
            ui.text(format!("Process RSS: {:.2} MiB", mib(rss)));
            ui.text_disabled(format!("non-Rust (C++/driver/libs): {:.2} MiB", mib(rss.saturating_sub(m.heap_bytes))));
        }
        None => ui.text_disabled("RSS unavailable on this OS"),
    }
}
