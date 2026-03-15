use super::models::SyncStormMetrics;
use super::profile::SyncStormConfig;
use super::runtime::{
    SYNC_BUNDLE_ENV, dispatch_result, duration_ms, json_f64, json_string, json_u64,
    load_client_from_env,
};
use serde_json::{Value as JsonValue, json};
use std::thread;
use std::time::{Duration, Instant};

const SYNC_TIMEOUT: Duration = Duration::from_secs(30);
const SYNC_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) fn run_sync_storm_benchmark(
    config: &SyncStormConfig,
) -> Result<SyncStormMetrics, String> {
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
        if let Some(summary) = matched_summary(&status, &scenario_id)? {
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

        if Instant::now() >= deadline {
            return Err(format!(
                "sync benchmark timed out waiting for scenario {scenario_id}"
            ));
        }

        thread::sleep(SYNC_POLL_INTERVAL);
    }
}

fn matched_summary<'a>(
    status: &'a JsonValue,
    scenario_id: &str,
) -> Result<Option<&'a JsonValue>, String> {
    let active_scenario = status
        .get("activeScenarioId")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string);
    let last_summary = status
        .get("lastSummary")
        .and_then(|value| if value.is_null() { None } else { Some(value) });

    if active_scenario.is_none()
        && let Some(summary) = last_summary
    {
        let summary_id = json_string(summary, "scenarioId")?;
        if summary_id == scenario_id {
            return Ok(Some(summary));
        }
    }

    Ok(None)
}
