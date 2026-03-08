use serde_json::Value;
use volt_core::webview::{WebViewConfig, WebViewSource};
use volt_core::window::WindowConfig;

/// Parse a JSON value into a WindowConfig.
pub(super) fn parse_window_config(config: &Value) -> WindowConfig {
    let window = config.get("window").unwrap_or(config);

    WindowConfig {
        title: window
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Volt")
            .to_string(),
        width: window
            .get("width")
            .and_then(|v| v.as_f64())
            .unwrap_or(800.0),
        height: window
            .get("height")
            .and_then(|v| v.as_f64())
            .unwrap_or(600.0),
        min_width: window.get("minWidth").and_then(|v| v.as_f64()),
        min_height: window.get("minHeight").and_then(|v| v.as_f64()),
        max_width: window.get("maxWidth").and_then(|v| v.as_f64()),
        max_height: window.get("maxHeight").and_then(|v| v.as_f64()),
        resizable: window
            .get("resizable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        decorations: window
            .get("decorations")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        transparent: window
            .get("transparent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        always_on_top: window
            .get("alwaysOnTop")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        maximized: window
            .get("maximized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        visible: window
            .get("visible")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        x: window.get("x").and_then(|v| v.as_f64()),
        y: window.get("y").and_then(|v| v.as_f64()),
    }
}

/// Parse a JSON value into a WebViewConfig.
pub(super) fn parse_webview_config(config: &Value) -> WebViewConfig {
    let url = config
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let html = config
        .get("html")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source = if let Some(url) = url {
        WebViewSource::Url(url)
    } else if let Some(html) = html {
        WebViewSource::Html(html)
    } else {
        WebViewSource::Html("<html><body><h1>Volt</h1></body></html>".to_string())
    };

    let allowed_origins = config
        .get("allowedOrigins")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    WebViewConfig {
        source,
        devtools: config
            .get("devtools")
            .and_then(|v| v.as_bool())
            .unwrap_or(cfg!(debug_assertions)),
        transparent: config
            .get("transparent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        user_agent: config
            .get("userAgent")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        allowed_origins,
        init_script: config
            .get("initScript")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}
