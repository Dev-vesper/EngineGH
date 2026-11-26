//! Parent-process side: runs every scenario in its own child process (clean memory counters,
//! clean GPU state) and merges the results into one JSON report.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::Matrix;
use crate::report::{ChildOutput, EnvironmentReport, Report, ScenarioReport, ScenarioSpec, SCHEMA_VERSION};

fn on_off(b: bool) -> &'static str {
    if b { "on" } else { "off" }
}

fn run_child(spec: &ScenarioSpec, m: &Matrix) -> Result<ChildOutput, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate own executable: {e}"))?;
    let output = Command::new(exe)
        .arg("--child")
        .args(["--instances", &spec.instances.to_string()])
        .args(["--msaa", &spec.msaa_samples.to_string()])
        .args(["--vsync", on_off(spec.vsync)])
        .args(["--gui", on_off(spec.gui)])
        .args(["--frames", &m.frames.to_string()])
        .args(["--warmup", &m.warmup.to_string()])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("failed to spawn child: {e}"))?;

    if !output.status.success() {
        return Err(format!("child exited with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().rev().find(|l| l.starts_with('{')).ok_or("child produced no JSON")?;
    serde_json::from_str(line).map_err(|e| format!("invalid child JSON: {e}"))
}

pub fn run(m: &Matrix) -> Result<PathBuf, String> {
    let scenarios = m.scenarios();
    let mut environment: Option<EnvironmentReport> = None;
    let mut results = Vec::with_capacity(scenarios.len());

    for (i, spec) in scenarios.iter().enumerate() {
        eprintln!(
            "[{}/{}] instances={} msaa={} vsync={} gui={}",
            i + 1,
            scenarios.len(),
            spec.instances,
            spec.msaa_samples,
            on_off(spec.vsync),
            on_off(spec.gui)
        );
        match run_child(spec, m) {
            Ok(out) => {
                let ms = &out.measurements;
                eprintln!(
                    "        {:.1} fps avg | 1% low {:.1} | p99 {:.2} ms | msaa x{}",
                    ms.fps_avg, ms.fps_1pct_low, ms.frame_interval_ms.p99, ms.effective_msaa_samples
                );
                environment.get_or_insert(out.environment);
                results.push(ScenarioReport { scenario: spec.clone(), measurements: Some(out.measurements), error: None });
            }
            Err(e) => {
                eprintln!("        FAILED: {e}");
                results.push(ScenarioReport { scenario: spec.clone(), measurements: None, error: Some(e) });
            }
        }
    }

    let timestamp_unix = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let report = Report {
        schema_version: SCHEMA_VERSION,
        benchmark_version: env!("CARGO_PKG_VERSION").to_owned(),
        timestamp_unix,
        environment,
        results,
    };

    let path = m
        .out
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("benchmarks/results/bench-{timestamp_unix}.json")));
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let file = File::create(&path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    serde_json::to_writer_pretty(BufWriter::new(file), &report).map_err(|e| format!("cannot write report: {e}"))?;

    if report.results.iter().any(|r| r.error.is_some()) {
        return Err(format!("some scenarios failed; partial report written to {}", path.display()));
    }
    Ok(path)
}
