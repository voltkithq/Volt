use super::*;

#[test]
fn test_menu_item_config_defaults() {
    let config: MenuItemConfig = serde_json::from_str(r#"{"label":"File"}"#).unwrap();
    assert_eq!(config.label, "File");
    assert_eq!(config.item_type, "normal");
    assert!(config.enabled);
    assert!(config.accelerator.is_none());
    assert!(config.role.is_none());
    assert!(config.submenu.is_empty());
}

#[test]
fn test_menu_item_config_separator() {
    let config: MenuItemConfig =
        serde_json::from_str(r#"{"label":"","item_type":"separator"}"#).unwrap();
    assert_eq!(config.item_type, "separator");
}

#[test]
fn test_menu_item_config_submenu() {
    let json = r#"{
        "label": "Edit",
        "item_type": "submenu",
        "submenu": [
            {"label": "Copy", "role": "copy"},
            {"label": "Paste", "role": "paste"}
        ]
    }"#;
    let config: MenuItemConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.item_type, "submenu");
    assert_eq!(config.submenu.len(), 2);
    assert_eq!(config.submenu[0].label, "Copy");
    assert_eq!(config.submenu[1].role.as_deref(), Some("paste"));
}

#[test]
fn test_menu_item_config_serde_roundtrip() {
    let config = MenuItemConfig {
        id: None,
        label: "Test".to_string(),
        accelerator: Some("CmdOrCtrl+T".to_string()),
        enabled: false,
        item_type: "normal".to_string(),
        role: Some("copy".to_string()),
        submenu: vec![],
    };
    let json = serde_json::to_string(&config).unwrap();
    let restored: MenuItemConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.label, "Test");
    assert_eq!(restored.accelerator.as_deref(), Some("CmdOrCtrl+T"));
    assert!(!restored.enabled);
    assert_eq!(restored.role.as_deref(), Some("copy"));
}

#[test]
fn test_menu_item_config_with_accelerator() {
    let config: MenuItemConfig =
        serde_json::from_str(r#"{"label":"Save","accelerator":"CmdOrCtrl+S"}"#).unwrap();
    assert_eq!(config.accelerator.as_deref(), Some("CmdOrCtrl+S"));
}

#[test]
fn test_parse_menu_accelerator_accepts_cmd_or_ctrl() {
    let parsed = super::accelerator::parse_menu_accelerator(Some("CmdOrCtrl+S"));
    assert!(parsed.is_ok());
    assert!(parsed.unwrap().is_some());
}

#[test]
fn test_parse_menu_accelerator_rejects_invalid_tokens() {
    let parsed = super::accelerator::parse_menu_accelerator(Some("CmdOrCtrl+DefinitelyNotAKey"));
    assert!(parsed.is_err());
}

#[test]
fn test_predefined_item_from_role_quit() {
    assert!(super::accelerator::predefined_item_from_role("quit").is_some());
}

#[test]
fn test_predefined_item_from_role_copy_cut_paste() {
    assert!(super::accelerator::predefined_item_from_role("copy").is_some());
    assert!(super::accelerator::predefined_item_from_role("cut").is_some());
    assert!(super::accelerator::predefined_item_from_role("paste").is_some());
}

#[test]
fn test_predefined_item_from_role_select_all_variants() {
    assert!(super::accelerator::predefined_item_from_role("selectAll").is_some());
    assert!(super::accelerator::predefined_item_from_role("select-all").is_some());
}

#[test]
fn test_predefined_item_from_role_undo_redo() {
    assert!(super::accelerator::predefined_item_from_role("undo").is_some());
    assert!(super::accelerator::predefined_item_from_role("redo").is_some());
}

#[test]
fn test_predefined_item_from_role_minimize() {
    assert!(super::accelerator::predefined_item_from_role("minimize").is_some());
}

#[test]
fn test_predefined_item_from_role_separator() {
    assert!(super::accelerator::predefined_item_from_role("separator").is_some());
}

#[test]
fn test_predefined_item_from_role_unknown() {
    assert!(super::accelerator::predefined_item_from_role("unknown-role").is_none());
    assert!(super::accelerator::predefined_item_from_role("").is_none());
    assert!(super::accelerator::predefined_item_from_role("QUIT").is_none());
}

#[test]
fn test_menu_error_creation_display() {
    let e = MenuError::Creation("init failed".to_string());
    let msg = e.to_string();
    assert!(msg.contains("create menu"));
    assert!(msg.contains("init failed"));
}

#[test]
fn test_menu_error_operation_display() {
    let e = MenuError::Operation("append failed".to_string());
    let msg = e.to_string();
    assert!(msg.contains("operation"));
    assert!(msg.contains("append failed"));
}

#[test]
fn test_build_menu_empty() {
    let menu = build_menu(&[]);
    assert!(menu.is_ok());
}

#[test]
fn test_build_menu_with_normal_item() {
    let items = vec![MenuItemConfig {
        id: None,
        label: "Test Item".to_string(),
        accelerator: None,
        enabled: true,
        item_type: "normal".to_string(),
        role: None,
        submenu: vec![],
    }];
    let menu = build_menu(&items);
    assert!(menu.is_ok());
}

#[test]
fn test_build_menu_with_separator() {
    let items = vec![MenuItemConfig {
        id: None,
        label: "".to_string(),
        accelerator: None,
        enabled: true,
        item_type: "separator".to_string(),
        role: None,
        submenu: vec![],
    }];
    let menu = build_menu(&items);
    assert!(menu.is_ok());
}

#[test]
fn test_build_menu_with_role_item() {
    let items = vec![MenuItemConfig {
        id: None,
        label: "Quit".to_string(),
        accelerator: None,
        enabled: true,
        item_type: "normal".to_string(),
        role: Some("quit".to_string()),
        submenu: vec![],
    }];
    let menu = build_menu(&items);
    assert!(menu.is_ok());
}

#[test]
fn test_build_menu_with_submenu() {
    let items = vec![MenuItemConfig {
        id: None,
        label: "Edit".to_string(),
        accelerator: None,
        enabled: true,
        item_type: "submenu".to_string(),
        role: None,
        submenu: vec![
            MenuItemConfig {
                id: None,
                label: "Copy".to_string(),
                accelerator: None,
                enabled: true,
                item_type: "normal".to_string(),
                role: Some("copy".to_string()),
                submenu: vec![],
            },
            MenuItemConfig {
                id: None,
                label: "".to_string(),
                accelerator: None,
                enabled: true,
                item_type: "separator".to_string(),
                role: None,
                submenu: vec![],
            },
            MenuItemConfig {
                id: None,
                label: "Paste".to_string(),
                accelerator: None,
                enabled: true,
                item_type: "normal".to_string(),
                role: Some("paste".to_string()),
                submenu: vec![],
            },
        ],
    }];
    let menu = build_menu(&items);
    assert!(menu.is_ok());
}

#[test]
fn test_build_menu_records_custom_id_mapping_for_dispatch() {
    let custom_id = "menu-custom-open".to_string();
    let items = vec![MenuItemConfig {
        id: Some(custom_id.clone()),
        label: "Open".to_string(),
        accelerator: None,
        enabled: true,
        item_type: "normal".to_string(),
        role: None,
        submenu: vec![],
    }];

    let menu = build_menu(&items);
    assert!(menu.is_ok());

    // Take a snapshot immediately after build_menu to avoid races with other
    // tests that also call build_menu (which replaces the global map).
    let mapping = super::build::menu_event_id_map_snapshot();
    assert!(mapping.values().any(|mapped| mapped == &custom_id));
    let internal_id = mapping
        .iter()
        .find_map(|(internal, mapped)| (mapped == &custom_id).then(|| internal.clone()))
        .expect("custom id should be present in mapping");

    // Verify via the snapshot (not the global resolve, which is racy under
    // parallel test runners like tarpaulin).
    assert_eq!(mapping.get(&internal_id).map(|s| s.as_str()), Some("menu-custom-open"));
}
