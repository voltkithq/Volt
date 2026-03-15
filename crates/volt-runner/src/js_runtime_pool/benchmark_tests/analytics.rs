use super::engine::BenchmarkEngine;
use super::models::AnalyticsStudioMetrics;
use super::profile::AnalyticsStudioConfig;
use super::runtime::{
    ANALYTICS_BUNDLE_ENV, dispatch_result, duration_ms, json_u64, load_client_from_env,
};
use serde_json::json;
use std::time::Instant;

pub(super) fn run_analytics_studio_benchmark(
    config: &AnalyticsStudioConfig,
    engine: BenchmarkEngine,
) -> Result<AnalyticsStudioMetrics, String> {
    let (_pool, client) = load_client_from_env(ANALYTICS_BUNDLE_ENV)?;

    let _profile = dispatch_result(
        &client,
        "analytics-profile",
        engine.analytics_profile_method(),
        engine.payload(json!({ "datasetSize": config.dataset_size })),
    )?;

    let started_at = Instant::now();
    let payload = dispatch_result(
        &client,
        "analytics-run",
        engine.analytics_run_method(),
        engine.payload(json!({
            "datasetSize": config.dataset_size,
            "iterations": config.iterations,
            "searchTerm": config.search_term,
            "minScore": config.min_score,
            "topN": config.top_n,
        })),
    )?;

    Ok(AnalyticsStudioMetrics {
        dataset_size: json_u64(&payload, "datasetSize")?,
        iterations: json_u64(&payload, "iterations")?,
        backend_duration_ms: json_u64(&payload, "backendDurationMs")?,
        round_trip_ms: duration_ms(started_at.elapsed()),
        peak_matches: json_u64(&payload, "peakMatches")?,
        payload_bytes: json_u64(&payload, "payloadBytes")?,
    })
}
