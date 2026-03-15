use serde_json::Value;

use super::{DEFAULT_WEBVIEW_URL, RunnerConfig, RunnerError};

mod plugins;
mod scalars;
mod webview;
mod window;

use self::plugins::parse_plugin_config;
use self::webview::parse_webview_config;
use self::window::parse_window_config;

pub(super) fn parse_runner_config_bytes(bytes: &[u8]) -> Result<RunnerConfig, RunnerError> {
    let parsed: Value = serde_json::from_slice(bytes).map_err(RunnerError::Json)?;
    if !parsed.is_object() {
        return Err(RunnerError::Config(
            "runner config must be a JSON object".to_string(),
        ));
    }
    parse_runner_config_value(&parsed)
}

fn parse_runner_config_value(parsed: &Value) -> Result<RunnerConfig, RunnerError> {
    let app_name = parsed
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Volt App")
        .to_string();
    let devtools = parsed
        .get("devtools")
        .and_then(Value::as_bool)
        .unwrap_or(cfg!(debug_assertions));
    let permissions = parsed
        .get("permissions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let fs_base_dir = parsed
        .get("fsBaseDir")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            parsed
                .get("baseDir")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        });
    let runtime_pool_size = parsed
        .get("runtime")
        .and_then(Value::as_object)
        .and_then(|runtime| runtime.get("poolSize"))
        .or_else(|| parsed.get("runtimePoolSize"))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    let updater_telemetry_enabled = parsed
        .get("updater")
        .and_then(Value::as_object)
        .and_then(|updater| updater.get("telemetry"))
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let updater_telemetry_sink = parsed
        .get("updater")
        .and_then(Value::as_object)
        .and_then(|updater| updater.get("telemetry"))
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get("sink"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let plugins = parse_plugin_config(parsed.get("plugins"))?;

    let window = parse_window_config(parsed.get("window").unwrap_or(parsed));
    let webview =
        parse_webview_config(parsed.get("webview").unwrap_or(parsed), DEFAULT_WEBVIEW_URL)?;

    Ok(RunnerConfig {
        app_name,
        devtools,
        permissions,
        fs_base_dir,
        runtime_pool_size,
        updater_telemetry_enabled,
        updater_telemetry_sink,
        plugins,
        window,
        webview,
    })
}
