use serde_json::Value;
use volt_core::webview::{WebViewConfig, WebViewSource};
use volt_core::window::WindowConfig;

use super::{DEFAULT_WEBVIEW_URL, RunnerConfig, RunnerError};

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
        window,
        webview,
    })
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
