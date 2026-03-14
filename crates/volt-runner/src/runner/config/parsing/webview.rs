use serde_json::Value;
use volt_core::webview::{WebViewConfig, WebViewSource};

use super::super::RunnerError;

pub(super) fn parse_webview_config(
    parsed: &Value,
    default_webview_url: &str,
) -> Result<WebViewConfig, RunnerError> {
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
        WebViewSource::Url(default_webview_url.to_string())
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
