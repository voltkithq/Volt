use crate::app;

#[test]
fn test_app_config_default() {
    let config = app::AppConfig::default();
    assert_eq!(config.name, "Volt App");
    assert_eq!(config.devtools, cfg!(debug_assertions));
}

#[test]
fn test_app_config_custom() {
    let config = app::AppConfig {
        name: "My Desktop App".to_string(),
        devtools: false,
    };
    assert_eq!(config.name, "My Desktop App");
    assert!(!config.devtools);
}

#[test]
fn test_app_config_clone() {
    let config = app::AppConfig {
        name: "Clone Test".to_string(),
        devtools: true,
    };
    let cloned = config.clone();
    assert_eq!(cloned.name, "Clone Test");
    assert!(cloned.devtools);
}

#[test]
fn test_app_config_debug() {
    let config = app::AppConfig::default();
    let debug = format!("{config:?}");
    assert!(debug.contains("Volt App"));
}

#[test]
fn test_app_error_event_loop_creation_display() {
    let error = app::AppError::EventLoopCreation("no display".to_string());
    let message = error.to_string();
    assert!(message.contains("event loop"));
    assert!(message.contains("no display"));
}

#[test]
fn test_app_error_window_creation_display() {
    let error = app::AppError::WindowCreation("gpu error".to_string());
    let message = error.to_string();
    assert!(message.contains("window"));
    assert!(message.contains("gpu error"));
}

#[test]
fn test_app_error_webview_creation_display() {
    let error = app::AppError::WebViewCreation("webview2 missing".to_string());
    let message = error.to_string();
    assert!(message.contains("webview"));
    assert!(message.contains("webview2 missing"));
}

#[test]
fn test_app_error_event_loop_consumed_display() {
    let error = app::AppError::EventLoopConsumed;
    assert!(error.to_string().contains("consumed"));
}

#[test]
fn test_app_error_generic_display() {
    let error = app::AppError::Generic("unknown failure".to_string());
    assert!(error.to_string().contains("unknown failure"));
}
