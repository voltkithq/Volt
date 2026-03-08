use volt_core::webview::{WebViewConfig, WebViewSource};
use volt_core::window::WindowConfig;

use super::*;

const DEFAULT_CONFIG_BYTES: &[u8] = include_bytes!("../../../assets/default-config.json");

fn config_with_app_name(name: &str) -> RunnerConfig {
    RunnerConfig {
        app_name: name.to_string(),
        devtools: false,
        permissions: Vec::new(),
        fs_base_dir: None,
        runtime_pool_size: None,
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
        window: WindowConfig::default(),
        webview: WebViewConfig::default(),
    }
}

#[test]
fn embedded_runner_config_parses_successfully() {
    let parsed =
        parsing::parse_runner_config_bytes(EMBEDDED_CONFIG_BYTES).expect("embedded config");
    assert!(!parsed.app_name.trim().is_empty());
}

#[test]
fn embedded_config_parses_expected_window_values() {
    let parsed = parsing::parse_runner_config_bytes(DEFAULT_CONFIG_BYTES).expect("embedded config");
    assert_eq!(parsed.app_name, "Volt Runner");
    assert!(!parsed.devtools);
    assert_eq!(parsed.window.title, "Volt Runner");
    assert_eq!(parsed.window.width, 1024.0);
    assert_eq!(parsed.window.height, 720.0);

    match parsed.webview.source {
        WebViewSource::Url(url) => assert_eq!(url, DEFAULT_WEBVIEW_URL),
        WebViewSource::Html(_) => panic!("expected URL source"),
    }
}

#[test]
fn config_defaults_are_applied_when_fields_are_missing() {
    let parsed = parsing::parse_runner_config_bytes(br#"{}"#).expect("default config");
    assert_eq!(parsed.app_name, "Volt App");
    assert_eq!(parsed.window.title, "Volt");
    assert_eq!(parsed.window.width, 800.0);
    assert_eq!(parsed.window.height, 600.0);
    assert!(parsed.runtime_pool_size.is_none());
    assert!(!parsed.updater_telemetry_enabled);
    assert!(parsed.updater_telemetry_sink.is_none());

    match parsed.webview.source {
        WebViewSource::Url(url) => assert_eq!(url, DEFAULT_WEBVIEW_URL),
        WebViewSource::Html(_) => panic!("expected URL source"),
    }
}

#[test]
fn config_parses_runtime_pool_size() {
    let parsed = parsing::parse_runner_config_bytes(
        br#"{
                "runtime": {
                    "poolSize": 4
                }
            }"#,
    )
    .expect("runtime pool config");
    assert_eq!(parsed.runtime_pool_size, Some(4));
}

#[test]
fn config_parses_updater_telemetry_controls() {
    let parsed = parsing::parse_runner_config_bytes(
        br#"{
                "updater": {
                    "telemetry": {
                        "enabled": true,
                        "sink": "stdout"
                    }
                }
            }"#,
    )
    .expect("telemetry config");
    assert!(parsed.updater_telemetry_enabled);
    assert_eq!(parsed.updater_telemetry_sink.as_deref(), Some("stdout"));
}

#[test]
fn config_supports_html_webview_source() {
    let parsed = parsing::parse_runner_config_bytes(
        br#"{
                "webview": {
                    "html": "<html><body>ok</body></html>",
                    "devtools": true
                }
            }"#,
    )
    .expect("valid html source");

    match parsed.webview.source {
        WebViewSource::Html(html) => assert!(html.contains("ok")),
        WebViewSource::Url(_) => panic!("expected HTML source"),
    }
    assert!(parsed.webview.devtools);
}

#[test]
fn invalid_config_json_is_rejected() {
    let err = parsing::parse_runner_config_bytes(br#"{invalid"#).expect_err("invalid json");
    assert!(matches!(err, RunnerError::Json(_)));
}

#[test]
fn non_object_config_payload_is_rejected() {
    let err = parsing::parse_runner_config_bytes(br#"[]"#).expect_err("invalid config shape");
    assert!(matches!(err, RunnerError::Config(_)));
}

#[test]
fn empty_webview_url_is_rejected() {
    let err = parsing::parse_runner_config_bytes(
        br#"{
                "webview": {
                    "url": "   "
                }
            }"#,
    )
    .expect_err("invalid webview url");

    assert!(matches!(err, RunnerError::Config(_)));
}

#[test]
fn app_name_override_applies_non_empty_values() {
    let mut config = config_with_app_name("Base");

    apply_app_name_override(&mut config, Ok("Custom Name".to_string())).expect("override");
    assert_eq!(config.app_name, "Custom Name");
}

#[test]
fn app_name_override_ignores_empty_values() {
    let mut config = config_with_app_name("Base");

    apply_app_name_override(&mut config, Ok("   ".to_string())).expect("no override");
    assert_eq!(config.app_name, "Base");
}
