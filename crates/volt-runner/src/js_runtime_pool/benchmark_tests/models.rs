use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HeadlessBenchmarkSummary {
    pub(super) analytics_studio: BenchmarkCase<AnalyticsStudioMetrics>,
    pub(super) sync_storm: BenchmarkCase<SyncStormMetrics>,
    pub(super) workflow_lab: BenchmarkCase<WorkflowLabMetrics>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BenchmarkCase<T> {
    pub(super) status: String,
    pub(super) error: Option<String>,
    pub(super) metrics: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnalyticsStudioMetrics {
    pub(super) dataset_size: u64,
    pub(super) iterations: u64,
    pub(super) backend_duration_ms: u64,
    pub(super) round_trip_ms: u64,
    pub(super) peak_matches: u64,
    pub(super) payload_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SyncStormMetrics {
    pub(super) worker_count: u64,
    pub(super) ticks_per_worker: u64,
    pub(super) total_tick_events: u64,
    pub(super) snapshot_events: u64,
    pub(super) backend_duration_ms: u64,
    pub(super) round_trip_ms: u64,
    pub(super) average_drift_ms: f64,
    pub(super) max_drift_ms: u64,
    pub(super) queue_peak: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WorkflowLabMetrics {
    pub(super) batch_size: u64,
    pub(super) passes: u64,
    pub(super) pipeline_length: u64,
    pub(super) backend_duration_ms: u64,
    pub(super) round_trip_ms: u64,
    pub(super) payload_bytes: u64,
}
