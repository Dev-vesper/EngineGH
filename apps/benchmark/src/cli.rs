use std::path::PathBuf;
use std::str::FromStr;

use crate::report::ScenarioSpec;

pub const USAGE: &str = "\
benchmark - end-to-end engine render benchmark (JSON report)

USAGE:
    benchmark [OPTIONS]

OPTIONS (comma-separated lists produce the full cross product, one process per scenario):
    --instances <N,..>   cube counts                 [default: 10000,50000,200000]
    --msaa <1|2|4|8,..>  requested MSAA samples      [default: 1,4]
    --vsync <on|off,..>                              [default: off]
    --gui <on|off,..>    Dear ImGui overlay (needs --features editor)  [default: off]
    --frames <N>         measured frames per scenario [default: 600]
    --warmup <N>         warm-up frames, not measured [default: 120]
    --out <PATH>         report path [default: benchmarks/results/bench-<unix-time>.json]
    -h, --help

Run with --release; debug numbers are not meaningful.
If several results sit at your monitor's refresh rate, those runs are display-bound:
use larger --instances so the GPU, not the display, is the limit.";

#[derive(Clone, Debug)]
pub struct Matrix {
    pub instances: Vec<u32>,
    pub msaa: Vec<u32>,
    pub vsync: Vec<bool>,
    pub gui: Vec<bool>,
    pub frames: u32,
    pub warmup: u32,
    pub out: Option<PathBuf>,
}

impl Default for Matrix {
    fn default() -> Self {
        Self {
            instances: vec![10_000, 50_000, 200_000],
            msaa: vec![1, 4],
            vsync: vec![false],
            gui: vec![false],
            frames: 600,
            warmup: 120,
            out: None,
        }
    }
}

impl Matrix {
    pub fn scenarios(&self) -> Vec<ScenarioSpec> {
        let mut v = Vec::new();
        for &instances in &self.instances {
            for &msaa_samples in &self.msaa {
                for &vsync in &self.vsync {
                    for &gui in &self.gui {
                        v.push(ScenarioSpec { instances, msaa_samples, vsync, gui });
                    }
                }
            }
        }
        v
    }
}

pub struct Cli {
    /// Internal: run exactly one scenario in this process and print its JSON on stdout.
    pub child: bool,
    pub matrix: Matrix,
}

fn list<T: FromStr>(flag: &str, value: &str) -> Result<Vec<T>, String> {
    let items: Result<Vec<T>, _> = value.split(',').map(|s| s.trim().parse::<T>()).collect();
    items.map_err(|_| format!("invalid value for {flag}: '{value}'"))
}

fn on_off(flag: &str, value: &str) -> Result<Vec<bool>, String> {
    value
        .split(',')
        .map(|s| match s.trim() {
            "on" => Ok(true),
            "off" => Ok(false),
            _ => Err(format!("{flag} expects on/off, got '{value}'")),
        })
        .collect()
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Cli, String> {
    let mut m = Matrix::default();
    let mut child = false;
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        if flag == "--child" {
            child = true;
            continue;
        }
        if flag == "-h" || flag == "--help" {
            return Err(String::new());
        }
        let value = it.next().ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--instances" => m.instances = list(&flag, &value)?,
            "--msaa" => m.msaa = list(&flag, &value)?,
            "--vsync" => m.vsync = on_off(&flag, &value)?,
            "--gui" => m.gui = on_off(&flag, &value)?,
            "--frames" => m.frames = value.parse().map_err(|_| format!("invalid --frames '{value}'"))?,
            "--warmup" => m.warmup = value.parse().map_err(|_| format!("invalid --warmup '{value}'"))?,
            "--out" => m.out = Some(PathBuf::from(value)),
            other => return Err(format!("unknown option '{other}'")),
        }
    }
    if m.instances.iter().any(|&n| n == 0) {
        return Err("--instances values must be > 0".into());
    }
    if m.msaa.iter().any(|n| ![1, 2, 4, 8].contains(n)) {
        return Err("--msaa values must be 1, 2, 4 or 8".into());
    }
    if m.frames == 0 {
        return Err("--frames must be > 0".into());
    }
    if child && m.scenarios().len() != 1 {
        return Err("--child needs exactly one value per list".into());
    }
    Ok(Cli { child, matrix: m })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Cli, String> {
        parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn cross_product() {
        let cli = p("--instances 10,20 --msaa 1,4 --vsync on,off").unwrap();
        assert_eq!(cli.matrix.scenarios().len(), 8);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(p("--msaa 3").is_err());
        assert!(p("--instances 0").is_err());
        assert!(p("--vsync maybe").is_err());
        assert!(p("--bogus 1").is_err());
        assert!(p("--child --instances 1,2").is_err());
    }
}
