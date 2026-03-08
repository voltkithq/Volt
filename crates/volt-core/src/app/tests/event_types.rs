use crate::app;

#[test]
fn test_app_event_quit() {
    let event = app::AppEvent::Quit;
    assert!(matches!(event, app::AppEvent::Quit));
}

#[test]
fn test_app_event_clone() {
    let event = app::AppEvent::ProcessCommands;
    let cloned = event.clone();
    assert!(matches!(cloned, app::AppEvent::ProcessCommands));
}

#[test]
fn test_app_event_debug() {
    let event = app::AppEvent::Quit;
    let debug = format!("{event:?}");
    assert!(debug.contains("Quit"));
}

#[test]
fn test_app_event_create_window() {
    let event = app::AppEvent::CreateWindow {
        window_config: Box::new(crate::window::WindowConfig::default()),
        webview_config: Box::new(crate::webview::WebViewConfig::default()),
        js_window_id: Some("window-1".to_string()),
    };
    assert!(matches!(event, app::AppEvent::CreateWindow { .. }));
}
