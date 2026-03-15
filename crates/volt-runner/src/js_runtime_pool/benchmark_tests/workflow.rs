use super::engine::BenchmarkEngine;
use super::models::WorkflowLabMetrics;
use super::profile::WorkflowLabConfig;
use super::runtime::{
    WORKFLOW_BUNDLE_ENV, dispatch_result, duration_ms, json_u64, load_client_from_env,
};
use serde_json::{Value as JsonValue, json};
use std::time::Instant;

pub(super) fn run_workflow_lab_benchmark(
    config: &WorkflowLabConfig,
    engine: BenchmarkEngine,
) -> Result<WorkflowLabMetrics, String> {
    let (_pool, client) = load_client_from_env(WORKFLOW_BUNDLE_ENV)?;

    let started_at = Instant::now();
    let payload = dispatch_result(
        &client,
        "workflow-run",
        engine.workflow_run_method(),
        engine.payload(json!({
            "batchSize": config.batch_size,
            "passes": config.passes,
        })),
    )?;
    let pipeline = payload
        .get("pipeline")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "workflow result missing pipeline array".to_string())?;

    Ok(WorkflowLabMetrics {
        batch_size: json_u64(&payload, "batchSize")?,
        passes: json_u64(&payload, "passes")?,
        pipeline_length: pipeline.len() as u64,
        backend_duration_ms: json_u64(&payload, "backendDurationMs")?,
        round_trip_ms: duration_ms(started_at.elapsed()),
        payload_bytes: json_u64(&payload, "payloadBytes")?,
    })
}
