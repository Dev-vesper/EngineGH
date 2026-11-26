mod cli;
mod orchestrator;
mod report;
mod runner;
mod scenario;
mod stats;

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;

use engine_platform::WindowConfig;

/// Heap numbers in the report come from this allocator (cost: two relaxed atomics per allocation).
#[global_allocator]
static ALLOCATOR: engine_memory::TrackingAllocator = engine_memory::TrackingAllocator;

fn run_child(cli: cli::Cli) -> ExitCode {
    let spec = cli.matrix.scenarios().remove(0);
    let outcome: runner::Outcome = Rc::new(RefCell::new(None));
    let app = runner::BenchApp::new(spec, cli.matrix.frames, cli.matrix.warmup, outcome.clone());
    let config = WindowConfig { title: "Engine Benchmark".to_owned(), ..Default::default() };

    if let Err(e) = engine_platform::run(config, app) {
        eprintln!("event loop error: {e}");
        return ExitCode::FAILURE;
    }
    let result = outcome.borrow_mut().take();
    match result {
        Some(Ok(out)) => match serde_json::to_string(&out) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("cannot serialise result: {e}");
                ExitCode::FAILURE
            }
        },
        Some(Err(e)) => {
            eprintln!("scenario failed: {e}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("window closed before the scenario finished");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = match cli::parse(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprintln!("{}", cli::USAGE);
            return if msg.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(2) };
        }
    };

    if cli.child {
        return run_child(cli);
    }
    if cfg!(debug_assertions) {
        eprintln!("warning: debug build - results are not representative. Use --release.");
    }
    match orchestrator::run(&cli.matrix) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
