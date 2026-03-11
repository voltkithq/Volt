use super::*;

#[test]
fn test_window_config_default() {
    let config = WindowConfig::default();
    assert_eq!(config.title, "Volt");
    assert_eq!(config.width, 800.0);
    assert_eq!(config.height, 600.0);
    assert!(config.resizable);
    assert!(config.decorations);
    assert!(!config.transparent);
    assert!(!config.always_on_top);
    assert!(!config.maximized);
    assert!(config.visible);
    assert!(config.x.is_none());
    assert!(config.y.is_none());
    assert!(config.min_width.is_none());
    assert!(config.min_height.is_none());
    assert!(config.max_width.is_none());
    assert!(config.max_height.is_none());
}

#[test]
fn test_window_config_serde_roundtrip() {
    let config = WindowConfig {
        title: "Test Window".to_string(),
        width: 1024.0,
        height: 768.0,
        min_width: Some(400.0),
        min_height: Some(300.0),
        max_width: Some(1920.0),
        max_height: Some(1080.0),
        resizable: false,
        decorations: false,
        transparent: true,
        always_on_top: true,
        maximized: true,
        visible: false,
        x: Some(100.0),
        y: Some(200.0),
        icon_rgba: None,
        icon_width: 0,
        icon_height: 0,
    };
    let json = serde_json::to_string(&config).expect("serialize config");
    let restored: WindowConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(restored.title, "Test Window");
    assert_eq!(restored.width, 1024.0);
    assert_eq!(restored.height, 768.0);
    assert_eq!(restored.min_width, Some(400.0));
    assert_eq!(restored.min_height, Some(300.0));
    assert_eq!(restored.max_width, Some(1920.0));
    assert_eq!(restored.max_height, Some(1080.0));
    assert!(!restored.resizable);
    assert!(!restored.decorations);
    assert!(restored.transparent);
    assert!(restored.always_on_top);
    assert!(restored.maximized);
    assert!(!restored.visible);
    assert_eq!(restored.x, Some(100.0));
    assert_eq!(restored.y, Some(200.0));
}

#[test]
fn test_window_config_serde_empty_json_uses_defaults() {
    let config: WindowConfig = serde_json::from_str("{}").expect("deserialize default config");
    assert_eq!(config.title, "Volt");
    assert_eq!(config.width, 800.0);
    assert_eq!(config.height, 600.0);
    assert!(config.resizable);
    assert!(config.decorations);
    assert!(config.visible);
    assert!(!config.transparent);
    assert!(!config.always_on_top);
    assert!(!config.maximized);
}

#[test]
fn test_window_config_serde_partial_json() {
    let config: WindowConfig =
        serde_json::from_str(r#"{"title":"Custom","width":1280}"#).expect("deserialize config");
    assert_eq!(config.title, "Custom");
    assert_eq!(config.width, 1280.0);
    assert_eq!(config.height, 600.0);
}

#[test]
fn test_window_config_with_position() {
    let config: WindowConfig =
        serde_json::from_str(r#"{"x":50.0,"y":75.0}"#).expect("deserialize config");
    assert_eq!(config.x, Some(50.0));
    assert_eq!(config.y, Some(75.0));
}

#[test]
fn test_window_config_with_min_max_dimensions() {
    let config: WindowConfig = serde_json::from_str(
        r#"{"min_width":320,"min_height":240,"max_width":3840,"max_height":2160}"#,
    )
    .expect("deserialize config");
    assert_eq!(config.min_width, Some(320.0));
    assert_eq!(config.min_height, Some(240.0));
    assert_eq!(config.max_width, Some(3840.0));
    assert_eq!(config.max_height, Some(2160.0));
}

#[test]
fn test_window_config_all_bool_flags() {
    let config: WindowConfig = serde_json::from_str(
        r#"{"resizable":false,"decorations":false,"transparent":true,"always_on_top":true,"maximized":true,"visible":false}"#,
    )
    .expect("deserialize config");
    assert!(!config.resizable);
    assert!(!config.decorations);
    assert!(config.transparent);
    assert!(config.always_on_top);
    assert!(config.maximized);
    assert!(!config.visible);
}

#[test]
fn test_window_config_clone() {
    let config = WindowConfig {
        title: "Clone Test".to_string(),
        ..WindowConfig::default()
    };
    let cloned = config.clone();
    assert_eq!(cloned.title, "Clone Test");
    assert_eq!(cloned.width, config.width);
}

#[test]
fn test_window_config_debug() {
    let config = WindowConfig::default();
    let debug = format!("{config:?}");
    assert!(debug.contains("Volt"));
    assert!(debug.contains("800"));
}

#[test]
fn test_window_config_accepts_partial_paired_fields() {
    let min_only: WindowConfig = serde_json::from_str(r#"{"min_width":320}"#).unwrap();
    assert_eq!(min_only.min_width, Some(320.0));
    assert_eq!(min_only.min_height, None);

    let max_only: WindowConfig = serde_json::from_str(r#"{"max_height":1080}"#).unwrap();
    assert_eq!(max_only.max_width, None);
    assert_eq!(max_only.max_height, Some(1080.0));

    let pos_only: WindowConfig = serde_json::from_str(r#"{"x":20}"#).unwrap();
    assert_eq!(pos_only.x, Some(20.0));
    assert_eq!(pos_only.y, None);
}

#[test]
fn test_window_error_build_display() {
    let error = WindowError::Build("GPU not available".to_string());
    assert!(error.to_string().contains("GPU not available"));
    assert!(error.to_string().contains("build window"));
}

#[test]
fn test_window_error_operation_display() {
    let error = WindowError::Operation("resize failed".to_string());
    assert!(error.to_string().contains("resize failed"));
    assert!(error.to_string().contains("operation"));
}
