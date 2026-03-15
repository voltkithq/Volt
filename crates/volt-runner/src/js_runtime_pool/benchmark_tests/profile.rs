use serde::Deserialize;
use std::env;

const BENCHMARK_PROFILE_ENV: &str = "VOLT_BENCH_PROFILE_JSON";

#[derive(Debug, Clone, Default)]
pub(super) struct BenchmarkProfile {
    pub(super) analytics_studio: AnalyticsStudioConfig,
    pub(super) sync_storm: SyncStormConfig,
    pub(super) workflow_lab: WorkflowLabConfig,
}

#[derive(Debug, Clone)]
pub(super) struct AnalyticsStudioConfig {
    pub(super) dataset_size: u64,
    pub(super) iterations: u64,
    pub(super) search_term: String,
    pub(super) min_score: u64,
    pub(super) top_n: u64,
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
pub(super) struct SyncStormConfig {
    pub(super) worker_count: u64,
    pub(super) ticks_per_worker: u64,
    pub(super) interval_ms: u64,
    pub(super) burst_size: u64,
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
pub(super) struct WorkflowLabConfig {
    pub(super) batch_size: u64,
    pub(super) passes: u64,
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

pub(super) fn load_benchmark_profile() -> Result<BenchmarkProfile, String> {
    let mut profile = BenchmarkProfile::default();
    let raw = match env::var(BENCHMARK_PROFILE_ENV) {
        Ok(raw) => raw,
        Err(env::VarError::NotPresent) => return Ok(profile),
        Err(error) => return Err(format!("failed to read {BENCHMARK_PROFILE_ENV}: {error}")),
    };

    let overrides: BenchmarkProfileOverrides = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {BENCHMARK_PROFILE_ENV}: {error}"))?;

    apply_analytics_overrides(&mut profile.analytics_studio, overrides.analytics_studio);
    apply_sync_overrides(&mut profile.sync_storm, overrides.sync_storm);
    apply_workflow_overrides(&mut profile.workflow_lab, overrides.workflow_lab);

    Ok(profile)
}

fn apply_analytics_overrides(
    config: &mut AnalyticsStudioConfig,
    overrides: Option<AnalyticsStudioOverrides>,
) {
    let Some(overrides) = overrides else {
        return;
    };

    if let Some(dataset_size) = overrides.dataset_size {
        config.dataset_size = dataset_size.max(1);
    }
    if let Some(iterations) = overrides.iterations {
        config.iterations = iterations.max(1);
    }
    if let Some(search_term) = overrides.search_term
        && !search_term.is_empty()
    {
        config.search_term = search_term;
    }
    if let Some(min_score) = overrides.min_score {
        config.min_score = min_score;
    }
    if let Some(top_n) = overrides.top_n {
        config.top_n = top_n.max(1);
    }
}

fn apply_sync_overrides(config: &mut SyncStormConfig, overrides: Option<SyncStormOverrides>) {
    let Some(overrides) = overrides else {
        return;
    };

    if let Some(worker_count) = overrides.worker_count {
        config.worker_count = worker_count.max(1);
    }
    if let Some(ticks_per_worker) = overrides.ticks_per_worker {
        config.ticks_per_worker = ticks_per_worker.max(1);
    }
    if let Some(interval_ms) = overrides.interval_ms {
        config.interval_ms = interval_ms.max(1);
    }
    if let Some(burst_size) = overrides.burst_size {
        config.burst_size = burst_size.max(1);
    }
}

fn apply_workflow_overrides(
    config: &mut WorkflowLabConfig,
    overrides: Option<WorkflowLabOverrides>,
) {
    let Some(overrides) = overrides else {
        return;
    };

    if let Some(batch_size) = overrides.batch_size {
        config.batch_size = batch_size.max(1);
    }
    if let Some(passes) = overrides.passes {
        config.passes = passes.max(1);
    }
}
