use serde_json::Value;
use volt_core::permissions::Permission;
use volt_core::webview::{WebViewConfig, WebViewSource};
use volt_core::window::WindowConfig;

use super::{
    DEFAULT_WEBVIEW_URL, RunnerConfig, RunnerError, RunnerPluginConfig, RunnerPluginLimits,
    RunnerPluginSpawning, RunnerPluginSpawningStrategy,
};

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
    let webview = parse_webview_config(parsed.get("webview").unwrap_or(parsed))?;

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

fn parse_plugin_config(value: Option<&Value>) -> Result<RunnerPluginConfig, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginConfig::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins must be an object when provided".to_string(),
        ));
    };

    let enabled = parse_string_array(object.get("enabled"), "plugins.enabled")?;
    let grants = parse_plugin_grants(object.get("grants"))?;
    let plugin_dirs = parse_string_array(object.get("pluginDirs"), "plugins.pluginDirs")?;
    let limits = parse_plugin_limits(object.get("limits"))?;
    let spawning = parse_plugin_spawning(object.get("spawning"))?;

    Ok(RunnerPluginConfig {
        enabled,
        grants,
        plugin_dirs,
        limits,
        spawning,
    })
}

fn parse_plugin_grants(
    value: Option<&Value>,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, RunnerError> {
    let Some(value) = value else {
        return Ok(std::collections::BTreeMap::new());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.grants must be an object when provided".to_string(),
        ));
    };

    let mut grants = std::collections::BTreeMap::new();
    for (plugin_id, granted) in object {
        if plugin_id.trim().is_empty() {
            return Err(RunnerError::Config(
                "plugins.grants keys must not be empty".to_string(),
            ));
        }
        let Some(values) = granted.as_array() else {
            return Err(RunnerError::Config(format!(
                "plugins.grants.{plugin_id} must be an array"
            )));
        };

        let mut parsed = Vec::with_capacity(values.len());
        for (index, entry) in values.iter().enumerate() {
            let Some(name) = entry.as_str() else {
                return Err(RunnerError::Config(format!(
                    "plugins.grants.{plugin_id}[{index}] must be a string"
                )));
            };
            if Permission::from_str_name(name).is_none() {
                return Err(RunnerError::Config(format!(
                    "plugins.grants.{plugin_id}[{index}] contains unknown permission '{name}'"
                )));
            }
            parsed.push(name.to_string());
        }
        grants.insert(plugin_id.to_string(), parsed);
    }

    Ok(grants)
}

fn parse_plugin_limits(value: Option<&Value>) -> Result<RunnerPluginLimits, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginLimits::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.limits must be an object when provided".to_string(),
        ));
    };

    Ok(RunnerPluginLimits {
        activation_timeout_ms: parse_positive_u64(
            object.get("activationTimeoutMs"),
            "plugins.limits.activationTimeoutMs",
            RunnerPluginLimits::default().activation_timeout_ms,
        )?,
        deactivation_timeout_ms: parse_positive_u64(
            object.get("deactivationTimeoutMs"),
            "plugins.limits.deactivationTimeoutMs",
            RunnerPluginLimits::default().deactivation_timeout_ms,
        )?,
        call_timeout_ms: parse_positive_u64(
            object.get("callTimeoutMs"),
            "plugins.limits.callTimeoutMs",
            RunnerPluginLimits::default().call_timeout_ms,
        )?,
        max_plugins: usize::try_from(parse_positive_u64(
            object.get("maxPlugins"),
            "plugins.limits.maxPlugins",
            RunnerPluginLimits::default().max_plugins as u64,
        )?)
        .map_err(|_| {
            RunnerError::Config("plugins.limits.maxPlugins is too large for this platform".into())
        })?,
        heartbeat_interval_ms: parse_positive_u64(
            object.get("heartbeatIntervalMs"),
            "plugins.limits.heartbeatIntervalMs",
            RunnerPluginLimits::default().heartbeat_interval_ms,
        )?,
        heartbeat_timeout_ms: parse_positive_u64(
            object.get("heartbeatTimeoutMs"),
            "plugins.limits.heartbeatTimeoutMs",
            RunnerPluginLimits::default().heartbeat_timeout_ms,
        )?,
    })
}

fn parse_plugin_spawning(value: Option<&Value>) -> Result<RunnerPluginSpawning, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginSpawning::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.spawning must be an object when provided".to_string(),
        ));
    };

    let strategy = match object.get("strategy").and_then(Value::as_str) {
        None => RunnerPluginSpawningStrategy::Lazy,
        Some("lazy") => RunnerPluginSpawningStrategy::Lazy,
        Some("eager") => RunnerPluginSpawningStrategy::Eager,
        Some(other) => {
            return Err(RunnerError::Config(format!(
                "plugins.spawning.strategy must be 'lazy' or 'eager', got '{other}'"
            )));
        }
    };

    Ok(RunnerPluginSpawning {
        strategy,
        idle_timeout_ms: parse_positive_u64(
            object.get("idleTimeoutMs"),
            "plugins.spawning.idleTimeoutMs",
            RunnerPluginSpawning::default().idle_timeout_ms,
        )?,
        pre_spawn: parse_string_array(object.get("preSpawn"), "plugins.spawning.preSpawn")?,
    })
}

fn parse_string_array(value: Option<&Value>, field: &str) -> Result<Vec<String>, RunnerError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(array) = value.as_array() else {
        return Err(RunnerError::Config(format!("{field} must be an array")));
    };

    let mut parsed = Vec::with_capacity(array.len());
    for (index, entry) in array.iter().enumerate() {
        let Some(text) = entry.as_str() else {
            return Err(RunnerError::Config(format!(
                "{field}[{index}] must be a string"
            )));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(RunnerError::Config(format!(
                "{field}[{index}] must not be empty"
            )));
        }
        parsed.push(trimmed.to_string());
    }

    Ok(parsed)
}

fn parse_positive_u64(
    value: Option<&Value>,
    field: &str,
    default: u64,
) -> Result<u64, RunnerError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let Some(number) = value.as_u64() else {
        return Err(RunnerError::Config(format!(
            "{field} must be a positive integer"
        )));
    };
    if number == 0 {
        return Err(RunnerError::Config(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(number)
}

/// Load a PNG file and decode it to RGBA pixel data.
fn load_icon_rgba(path: &str) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}

fn parse_window_config(parsed: &Value) -> WindowConfig {
    let (icon_rgba, icon_width, icon_height) = parsed
        .get("icon")
        .and_then(Value::as_str)
        .and_then(load_icon_rgba)
        .map(|(data, w, h)| (Some(data), w, h))
        .unwrap_or((None, 0, 0));

    WindowConfig {
        title: parsed
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Volt")
            .to_string(),
        width: parsed.get("width").and_then(Value::as_f64).unwrap_or(800.0),
        height: parsed
            .get("height")
            .and_then(Value::as_f64)
            .unwrap_or(600.0),
        min_width: parsed.get("minWidth").and_then(Value::as_f64),
        min_height: parsed.get("minHeight").and_then(Value::as_f64),
        max_width: parsed.get("maxWidth").and_then(Value::as_f64),
        max_height: parsed.get("maxHeight").and_then(Value::as_f64),
        resizable: parsed
            .get("resizable")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        decorations: parsed
            .get("decorations")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        transparent: parsed
            .get("transparent")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        always_on_top: parsed
            .get("alwaysOnTop")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        maximized: parsed
            .get("maximized")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        visible: parsed
            .get("visible")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        x: parsed.get("x").and_then(Value::as_f64),
        y: parsed.get("y").and_then(Value::as_f64),
        icon_rgba,
        icon_width,
        icon_height,
    }
}

fn parse_webview_config(parsed: &Value) -> Result<WebViewConfig, RunnerError> {
    let source = if let Some(url) = parsed.get("url").and_then(Value::as_str) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(RunnerError::Config(
                "webview.url must not be empty".to_string(),
            ));
        }
        WebViewSource::Url(trimmed.to_string())
    } else if let Some(html) = parsed.get("html").and_then(Value::as_str) {
        WebViewSource::Html(html.to_string())
    } else {
        WebViewSource::Url(DEFAULT_WEBVIEW_URL.to_string())
    };

    let allowed_origins = parsed
        .get("allowedOrigins")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();

    Ok(WebViewConfig {
        source,
        devtools: parsed
            .get("devtools")
            .and_then(Value::as_bool)
            .unwrap_or(cfg!(debug_assertions)),
        transparent: parsed
            .get("transparent")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        user_agent: parsed
            .get("userAgent")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        allowed_origins,
        init_script: parsed
            .get("initScript")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    })
}
