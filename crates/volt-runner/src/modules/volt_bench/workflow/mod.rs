use std::collections::BTreeMap;
use std::time::Instant;

use serde_json::Value;

mod data;
mod models;
mod pipeline;
mod util;

use self::data::build_documents;
pub use self::models::WorkflowBenchmarkOptions;
use self::models::{WorkflowBenchmarkResult, WorkflowStepTiming};
use self::pipeline::{normalize_pipeline, run_pipeline_step};
use self::util::{
    DEFAULT_BATCH_SIZE, DEFAULT_PASSES, clamp_positive_integer, duration_ms, serialized_len,
};

pub fn run_workflow_benchmark_json(options: WorkflowBenchmarkOptions) -> Result<Value, String> {
    let batch_size = clamp_positive_integer(options.batch_size, DEFAULT_BATCH_SIZE, 500, 25_000);
    let passes = clamp_positive_integer(options.passes, DEFAULT_PASSES, 1, 8);
    let pipeline = normalize_pipeline(options.pipeline);
    let mut documents = build_documents(batch_size);
    let mut step_timings = BTreeMap::<String, u64>::new();
    let started_at = Instant::now();

    for pass in 0..passes {
        for plugin in &pipeline {
            let plugin_started_at = Instant::now();
            run_pipeline_step(plugin, &mut documents, pass);
            *step_timings.entry(plugin.clone()).or_insert(0) +=
                duration_ms(plugin_started_at.elapsed());
        }
    }

    let mut route_distribution = BTreeMap::new();
    let mut total_priority = 0u64;
    for document in &documents {
        total_priority += document.priority;
        *route_distribution
            .entry(document.route.clone())
            .or_insert(0) += 1;
    }

    let mut result = WorkflowBenchmarkResult {
        batch_size,
        passes,
        pipeline: pipeline.clone(),
        backend_duration_ms: duration_ms(started_at.elapsed()),
        step_timings: step_timings
            .into_iter()
            .map(|(plugin, duration_ms)| WorkflowStepTiming {
                plugin,
                duration_ms,
            })
            .collect(),
        route_distribution,
        average_priority: if documents.is_empty() {
            0.0
        } else {
            ((total_priority as f64 / documents.len() as f64) * 100.0).round() / 100.0
        },
        digest_sample: documents
            .iter()
            .take(6)
            .map(|document| document.digest.clone())
            .collect(),
        payload_bytes: 0,
    };
    result.payload_bytes = serialized_len(&result)?;

    serde_json::to_value(result)
        .map_err(|error| format!("failed to serialize workflow benchmark result: {error}"))
}
