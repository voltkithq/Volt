use super::super::{WebViewConfig, WebViewError, WebViewSource};

#[test]
fn test_webview_config_default() {
    let config = WebViewConfig::default();
    assert!(matches!(config.source, WebViewSource::Html(_)));
    assert_eq!(config.devtools, cfg!(debug_assertions));
    assert!(!config.transparent);
    assert!(config.user_agent.is_none());
    assert!(config.allowed_origins.is_empty());
    assert!(config.init_script.is_none());
}

#[test]
fn test_webview_config_serde_roundtrip() {
    let config = WebViewConfig {
        source: WebViewSource::Url("http://localhost:5173".to_string()),
        devtools: true,
        transparent: false,
        user_agent: Some("VoltApp/1.0".to_string()),
        allowed_origins: vec!["https://api.example.com".to_string()],
        init_script: Some("console.log('init')".to_string()),
    };

    let json = serde_json::to_string(&config).expect("serialize config");
    let restored: WebViewConfig = serde_json::from_str(&json).expect("deserialize config");

    assert!(restored.devtools);
    assert_eq!(restored.user_agent.as_deref(), Some("VoltApp/1.0"));
    assert_eq!(restored.allowed_origins.len(), 1);
}

#[test]
fn test_webview_error_display() {
    let e = WebViewError::Build("backend error".into());
    assert!(e.to_string().contains("build webview"));

    let e = WebViewError::InvalidUrl("bad://url".into());
    assert!(e.to_string().contains("bad://url"));

    let e = WebViewError::NavigationBlocked("https://evil.com".into());
    assert!(e.to_string().contains("evil.com"));
}
