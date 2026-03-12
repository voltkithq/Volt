use std::env;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};

use super::*;

const ANALYTICS_BUNDLE_ENV: &str = "VOLT_BENCH_ANALYTICS_BUNDLE";
const SYNC_BUNDLE_ENV: &str = "VOLT_BENCH_SYNC_BUNDLE";
const WORKFLOW_BUNDLE_ENV: &str = "VOLT_BENCH_WORKFLOW_BUNDLE";
const BENCHMARK_PROFILE_ENV: &str = "VOLT_BENCH_PROFILE_JSON";
const BENCHMARK_ENGINE_ENV: &str = "VOLT_BENCH_ENGINE";

const IPC_TIMEOUT: Duration = Duration::from_secs(30);
const SYNC_TIMEOUT: Duration = Duration::from_secs(30);
const SYNC_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchmarkEngine {
    Js,
    Native,
    Direct,
}

impl BenchmarkEngine {
    fn from_env() -> Result<Self, String> {
        match env::var(BENCHMARK_ENGINE_ENV) {
            Ok(value) if value.eq_ignore_ascii_case("direct") => Ok(Self::Direct),
            Ok(value) if value.eq_ignore_ascii_case("native") => Ok(Self::Native),
            Ok(value) if value.eq_ignore_ascii_case("js") || value.trim().is_empty() => {
                Ok(Self::Js)
            }
            Ok(value) => Err(format!("unsupported {BENCHMARK_ENGINE_ENV} value: {value}")),
            Err(env::VarError::NotPresent) => Ok(Self::Js),
            Err(error) => Err(format!("failed to read {BENCHMARK_ENGINE_ENV}: {error}")),
        }
    }

    fn payload(self, payload: JsonValue) -> JsonValue {
        if !matches!(self, Self::Native) {
            return payload;
        }

        let mut payload = payload;
        if let Some(object) = payload.as_object_mut() {
            object.insert("engine".to_string(), json!("native"));
        }
        payload
    }

    fn analytics_profile_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::DATA_PROFILE_CHANNEL,
            Self::Js | Self::Native => "analytics:profile",
        }
    }

    fn analytics_run_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::DATA_QUERY_CHANNEL,
            Self::Js | Self::Native => "analytics:run",
        }
    }

    fn workflow_run_method(self) -> &'static str {
        match self {
            Self::Direct => crate::modules::volt_bench::WORKFLOW_RUN_CHANNEL,
            Self::Js | Self::Native => "workflow:run",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadlessBenchmarkSummary {
    analytics_studio: BenchmarkCase<AnalyticsStudioMetrics>,
    sync_storm: BenchmarkCase<SyncStormMetrics>,
    workflow_lab: BenchmarkCase<WorkflowLabMetrics>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkCase<T> {
    status: String,
    error: Option<String>,
    metrics: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsStudioMetrics {
    dataset_size: u64,
    iterations: u64,
    backend_duration_ms: u64,
    round_trip_ms: u64,
    peak_matches: u64,
    payload_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncStormMetrics {
    worker_count: u64,
    ticks_per_worker: u64,
    total_tick_events: u64,
    snapshot_events: u64,
    backend_duration_ms: u64,
    round_trip_ms: u64,
    average_drift_ms: f64,
    max_drift_ms: u64,
    queue_peak: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowLabMetrics {
    batch_size: u64,
    passes: u64,
    pipeline_length: u64,
    backend_duration_ms: u64,
    round_trip_ms: u64,
    payload_bytes: u64,
}

#[derive(Debug, Clone)]
struct BenchmarkProfile {
    analytics_studio: AnalyticsStudioConfig,
    sync_storm: SyncStormConfig,
    workflow_lab: WorkflowLabConfig,
}

impl Default for BenchmarkProfile {
    fn default() -> Self {
        Self {
            analytics_studio: AnalyticsStudioConfig::default(),
            sync_storm: SyncStormConfig::default(),
            workflow_lab: WorkflowLabConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct AnalyticsStudioConfig {
    dataset_size: u64,
    iterations: u64,
    search_term: String,
    min_score: u64,
    top_n: u64,
}

impl Default for AnalyticsStudioConfig {
    fn default() -> Self {
        Self {
            dataset_size: 50_000,
            iterations: 8,
            search_term: "risk".to_string(),
            min_score: 61,
            top_n: 24,
        }
    }
}

#[derive(Debug, Clone)]
struct SyncStormConfig {
    worker_count: u64,
    ticks_per_worker: u64,
    interval_ms: u64,
    burst_size: u64,
}

impl Default for SyncStormConfig {
    fn default() -> Self {
        Self {
            worker_count: 20,
            ticks_per_worker: 96,
            interval_ms: 2,
            burst_size: 8,
        }
    }
}

#[derive(Debug, Clone)]
struct WorkflowLabConfig {
    batch_size: u64,
    passes: u64,
}

impl Default for WorkflowLabConfig {
    fn default() -> Self {
        Self {
            batch_size: 6_000,
            passes: 4,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkProfileOverrides {
    analytics_studio: Option<AnalyticsStudioOverrides>,
    sync_storm: Option<SyncStormOverrides>,
    workflow_lab: Option<WorkflowLabOverrides>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsStudioOverrides {
    dataset_size: Option<u64>,
    iterations: Option<u64>,
    search_term: Option<String>,
    min_score: Option<u64>,
    top_n: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncStormOverrides {
    worker_count: Option<u64>,
    ticks_per_worker: Option<u64>,
    interval_ms: Option<u64>,
    burst_size: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowLabOverrides {
    batch_size: Option<u64>,
    passes: Option<u64>,
}

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

fn run_analytics_studio_benchmark(
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
    let round_trip_ms = duration_ms(started_at.elapsed());

    Ok(AnalyticsStudioMetrics {
        dataset_size: json_u64(&payload, "datasetSize")?,
        iterations: json_u64(&payload, "iterations")?,
        backend_duration_ms: json_u64(&payload, "backendDurationMs")?,
        round_trip_ms,
        peak_matches: json_u64(&payload, "peakMatches")?,
        payload_bytes: json_u64(&payload, "payloadBytes")?,
    })
}

fn run_sync_storm_benchmark(config: &SyncStormConfig) -> Result<SyncStormMetrics, String> {
    let (_pool, client) = load_client_from_env(SYNC_BUNDLE_ENV)?;

    let started_at = Instant::now();
    let run_payload = dispatch_result(
        &client,
        "sync-run",
        "sync:run",
        json!({
            "workerCount": config.worker_count,
            "ticksPerWorker": config.ticks_per_worker,
            "intervalMs": config.interval_ms,
            "burstSize": config.burst_size,
        }),
    )?;
    let scenario_id = json_string(&run_payload, "scenarioId")?;

    let deadline = Instant::now() + SYNC_TIMEOUT;
    loop {
        let status = dispatch_result(&client, "sync-status", "sync:status", JsonValue::Null)?;
        let active_scenario = status
            .get("activeScenarioId")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string);
        let last_summary = status
            .get("lastSummary")
            .and_then(|value| if value.is_null() { None } else { Some(value) });

        if active_scenario.is_none() {
            if let Some(summary) = last_summary {
                let summary_id = json_string(summary, "scenarioId")?;
                if summary_id == scenario_id {
                    return Ok(SyncStormMetrics {
                        worker_count: json_u64(summary, "workerCount")?,
                        ticks_per_worker: json_u64(summary, "ticksPerWorker")?,
                        total_tick_events: json_u64(summary, "totalTickEvents")?,
                        snapshot_events: json_u64(summary, "snapshotEvents")?,
                        backend_duration_ms: json_u64(summary, "backendDurationMs")?,
                        round_trip_ms: duration_ms(started_at.elapsed()),
                        average_drift_ms: json_f64(summary, "averageDriftMs")?,
                        max_drift_ms: json_u64(summary, "maxDriftMs")?,
                        queue_peak: json_u64(summary, "queuePeak")?,
                    });
                }
            }
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "sync benchmark timed out waiting for scenario {scenario_id}"
            ));
        }

        thread::sleep(SYNC_POLL_INTERVAL);
    }
}

fn run_workflow_lab_benchmark(
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
    let round_trip_ms = duration_ms(started_at.elapsed());
    let pipeline = payload
        .get("pipeline")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "workflow result missing pipeline array".to_string())?;

    Ok(WorkflowLabMetrics {
        batch_size: json_u64(&payload, "batchSize")?,
        passes: json_u64(&payload, "passes")?,
        pipeline_length: pipeline.len() as u64,
        backend_duration_ms: json_u64(&payload, "backendDurationMs")?,
        round_trip_ms,
        payload_bytes: json_u64(&payload, "payloadBytes")?,
    })
}

fn load_benchmark_profile() -> Result<BenchmarkProfile, String> {
    let mut profile = BenchmarkProfile::default();
    let raw = match env::var(BENCHMARK_PROFILE_ENV) {
        Ok(raw) => raw,
        Err(env::VarError::NotPresent) => return Ok(profile),
        Err(error) => return Err(format!("failed to read {BENCHMARK_PROFILE_ENV}: {error}")),
    };

    let overrides: BenchmarkProfileOverrides = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {BENCHMARK_PROFILE_ENV}: {error}"))?;

    if let Some(analytics) = overrides.analytics_studio {
        if let Some(dataset_size) = analytics.dataset_size {
            profile.analytics_studio.dataset_size = dataset_size.max(1);
        }
        if let Some(iterations) = analytics.iterations {
            profile.analytics_studio.iterations = iterations.max(1);
        }
        if let Some(search_term) = analytics.search_term {
            if !search_term.is_empty() {
                profile.analytics_studio.search_term = search_term;
            }
        }
        if let Some(min_score) = analytics.min_score {
            profile.analytics_studio.min_score = min_score;
        }
        if let Some(top_n) = analytics.top_n {
            profile.analytics_studio.top_n = top_n.max(1);
        }
    }

    if let Some(sync) = overrides.sync_storm {
        if let Some(worker_count) = sync.worker_count {
            profile.sync_storm.worker_count = worker_count.max(1);
        }
        if let Some(ticks_per_worker) = sync.ticks_per_worker {
            profile.sync_storm.ticks_per_worker = ticks_per_worker.max(1);
        }
        if let Some(interval_ms) = sync.interval_ms {
            profile.sync_storm.interval_ms = interval_ms.max(1);
        }
        if let Some(burst_size) = sync.burst_size {
            profile.sync_storm.burst_size = burst_size.max(1);
        }
    }

    if let Some(workflow) = overrides.workflow_lab {
        if let Some(batch_size) = workflow.batch_size {
            profile.workflow_lab.batch_size = batch_size.max(1);
        }
        if let Some(passes) = workflow.passes {
            profile.workflow_lab.passes = passes.max(1);
        }
    }

    Ok(profile)
}

fn load_client_from_env(bundle_env: &str) -> Result<(JsRuntimePool, JsRuntimePoolClient), String> {
    let bundle_path =
        env::var(bundle_env).map_err(|_| format!("missing required env var {bundle_env}"))?;
    let bundle_source = fs::read_to_string(&bundle_path)
        .map_err(|error| format!("failed to read bundle {bundle_path}: {error}"))?;

    let mut options = JsRuntimeOptions::default();
    options.permissions = vec!["fs".to_string()];
    options.app_name = "Volt Headless Benchmark".to_string();

    let pool = JsRuntimePool::start_with_options(2, options)?;
    let client = pool.client();
    client.load_backend_bundle(&bundle_source)?;

    Ok((pool, client))
}

fn dispatch_result(
    client: &JsRuntimePoolClient,
    id: &str,
    method: &str,
    args: JsonValue,
) -> Result<JsonValue, String> {
    let raw = json!({
        "id": id,
        "method": method,
        "args": args,
    })
    .to_string();

    let response = client.dispatch_ipc_message(&raw, IPC_TIMEOUT)?;
    let error = response.error;
    let error_code = response.error_code;
    let result = response.result;

    if let Some(error) = error {
        let code = error_code.unwrap_or_else(|| "unknown".to_string());
        return Err(format!("IPC {method} failed ({code}): {error}"));
    }

    result.ok_or_else(|| format!("IPC {method} returned no result payload"))
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn json_u64(value: &JsonValue, key: &str) -> Result<u64, String> {
    let entry = value
        .get(key)
        .ok_or_else(|| format!("missing numeric field {key}"))?;
    if let Some(number) = entry.as_u64() {
        return Ok(number);
    }
    if let Some(number) = entry.as_i64() {
        if number >= 0 {
            return Ok(number as u64);
        }
    }
    if let Some(number) = entry.as_f64() {
        if number.is_finite() && number >= 0.0 {
            return Ok(number.round() as u64);
        }
    }
    Err(format!("field {key} is not a non-negative number"))
}

fn json_f64(value: &JsonValue, key: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| format!("field {key} is not a number"))
}

fn json_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("field {key} is not a string"))
}
