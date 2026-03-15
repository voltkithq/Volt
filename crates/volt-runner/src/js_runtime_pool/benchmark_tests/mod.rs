mod analytics;
mod engine;
mod models;
mod profile;
mod runtime;
mod sync;
mod workflow;

use analytics::run_analytics_studio_benchmark;
use engine::BenchmarkEngine;
use models::{BenchmarkCase, HeadlessBenchmarkSummary};
use profile::load_benchmark_profile;
use sync::run_sync_storm_benchmark;
use workflow::run_workflow_lab_benchmark;

#[test]
#[ignore = "benchmark harness invoked explicitly by scripts/ci/volt-benchmarks.mjs"]
fn headless_example_backends_emit_benchmark_summary() {
    let summary = run_headless_example_backends();
    println!(
        "VOLT_BENCH_JSON:{}",
        serde_json::to_string(&summary).expect("serialize benchmark summary")
    );
}

fn run_headless_example_backends() -> HeadlessBenchmarkSummary {
    let profile = load_benchmark_profile().expect("parse benchmark profile overrides");
    let engine = BenchmarkEngine::from_env().expect("parse benchmark engine override");

    HeadlessBenchmarkSummary {
        analytics_studio: capture_case(|| {
            run_analytics_studio_benchmark(&profile.analytics_studio, engine)
        }),
        sync_storm: capture_case(|| run_sync_storm_benchmark(&profile.sync_storm)),
        workflow_lab: capture_case(|| run_workflow_lab_benchmark(&profile.workflow_lab, engine)),
    }
}

fn capture_case<T>(runner: impl FnOnce() -> Result<T, String>) -> BenchmarkCase<T> {
    match runner() {
        Ok(metrics) => BenchmarkCase {
            status: "ok".to_string(),
            error: None,
            metrics: Some(metrics),
        },
        Err(error) => {
            let status = if error.contains("timed out") || error.contains("IPC_HANDLER_TIMEOUT") {
                "timeout"
            } else {
                "error"
            };
            BenchmarkCase {
                status: status.to_string(),
                error: Some(error),
                metrics: None,
            }
        }
    }
}
