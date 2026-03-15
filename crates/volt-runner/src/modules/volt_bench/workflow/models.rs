use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowBenchmarkOptions {
    pub batch_size: Option<f64>,
    pub passes: Option<f64>,
    pub pipeline: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkflowStepTiming {
    pub(super) plugin: String,
    pub(super) duration_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkflowBenchmarkResult {
    pub(super) batch_size: u64,
    pub(super) passes: u64,
    pub(super) pipeline: Vec<String>,
    pub(super) backend_duration_ms: u64,
    pub(super) step_timings: Vec<WorkflowStepTiming>,
    pub(super) route_distribution: BTreeMap<String, u64>,
    pub(super) average_priority: f64,
    pub(super) digest_sample: Vec<String>,
    pub(super) payload_bytes: u64,
}
