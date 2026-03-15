use crate::js_runtime::JsRuntimeOptions;
use crate::js_runtime_pool::{JsRuntimePool, JsRuntimePoolClient};
use serde_json::{Value as JsonValue, json};
use std::env;
use std::fs;
use std::time::Duration;

pub(super) const ANALYTICS_BUNDLE_ENV: &str = "VOLT_BENCH_ANALYTICS_BUNDLE";
pub(super) const SYNC_BUNDLE_ENV: &str = "VOLT_BENCH_SYNC_BUNDLE";
pub(super) const WORKFLOW_BUNDLE_ENV: &str = "VOLT_BENCH_WORKFLOW_BUNDLE";

const IPC_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn load_client_from_env(
    bundle_env: &str,
) -> Result<(JsRuntimePool, JsRuntimePoolClient), String> {
    let bundle_path =
        env::var(bundle_env).map_err(|_| format!("missing required env var {bundle_env}"))?;
    let bundle_source = fs::read_to_string(&bundle_path)
        .map_err(|error| format!("failed to read bundle {bundle_path}: {error}"))?;

    let options = JsRuntimeOptions {
        permissions: vec!["fs".to_string()],
        app_name: "Volt Headless Benchmark".to_string(),
        ..JsRuntimeOptions::default()
    };

    let pool = JsRuntimePool::start_with_options(2, options)?;
    let client = pool.client();
    client.load_backend_bundle(&bundle_source)?;

    Ok((pool, client))
}

pub(super) fn dispatch_result(
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
    if let Some(error) = response.error {
        let code = response.error_code.unwrap_or_else(|| "unknown".to_string());
        return Err(format!("IPC {method} failed ({code}): {error}"));
    }

    response
        .result
        .ok_or_else(|| format!("IPC {method} returned no result payload"))
}

pub(super) fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

pub(super) fn json_u64(value: &JsonValue, key: &str) -> Result<u64, String> {
    let entry = value
        .get(key)
        .ok_or_else(|| format!("missing numeric field {key}"))?;
    if let Some(number) = entry.as_u64() {
        return Ok(number);
    }
    if let Some(number) = entry.as_i64()
        && number >= 0
    {
        return Ok(number as u64);
    }
    if let Some(number) = entry.as_f64()
        && number.is_finite()
        && number >= 0.0
    {
        return Ok(number.round() as u64);
    }

    Err(format!("field {key} is not a non-negative number"))
}

pub(super) fn json_f64(value: &JsonValue, key: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| format!("field {key} is not a number"))
}

pub(super) fn json_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("field {key} is not a string"))
}
