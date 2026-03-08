use super::*;
use crate::modules::test_utils::{init_test_bridge, shutdown_test_bridge, test_guard};
use std::thread;
use std::time::Duration;

#[test]
fn normalize_tray_event_name_accepts_click_only() {
    assert_eq!(
        normalize_tray_event_name("click".to_string()),
        Ok("tray:click")
    );
    assert!(normalize_tray_event_name("".to_string()).is_err());
    assert!(normalize_tray_event_name("open".to_string()).is_err());
}

#[test]
fn parse_tray_config_validates_shape() {
    assert!(parse_tray_config_value(&serde_json::json!({"tooltip": "ok"})).is_ok());
    assert!(parse_tray_config_value(&serde_json::json!(null)).is_err());
    assert!(parse_tray_config_value(&serde_json::json!({"tooltip": 42})).is_err());
    assert!(parse_tray_config_value(&serde_json::json!({"icon": ""})).is_err());
}

#[test]
fn icon_path_requires_filesystem_permission() {
    let temp_dir = std::env::temp_dir();
    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: temp_dir.clone(),
        permissions: vec!["tray".to_string()],
        ..Default::default()
    })
    .expect("configure module permissions");

    let denied = resolve_icon_path("icon.png");
    assert!(denied.is_err());
    assert!(
        denied
            .err()
            .is_some_and(|message| message.contains("Permission denied"))
    );

    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: temp_dir,
        permissions: vec!["tray".to_string(), "fs".to_string()],
        ..Default::default()
    })
    .expect("configure module permissions");

    let allowed = resolve_icon_path("icon.png");
    assert!(allowed.is_ok());
}

#[test]
fn tray_commands_dispatch_over_bridge() {
    let _guard = test_guard();
    let (receiver, lifecycle, _proxy) = init_test_bridge();
    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        permissions: vec!["tray".to_string()],
        ..Default::default()
    })
    .expect("configure module permissions");

    let responder = thread::spawn(move || {
        let first = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("create");
        match first.command {
            AppCommand::CreateTray { reply, .. } => {
                let _ = reply.send(Ok("tray-1".to_string()));
            }
            command => panic!("unexpected command: {command:?}"),
        }

        let second = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("set tooltip");
        match second.command {
            AppCommand::SetTrayTooltip { tooltip, reply } => {
                assert_eq!(tooltip, "Volt");
                let _ = reply.send(Ok(()));
            }
            command => panic!("unexpected command: {command:?}"),
        }

        let third = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("set visible");
        match third.command {
            AppCommand::SetTrayVisible { visible, reply } => {
                assert!(!visible);
                let _ = reply.send(Ok(()));
            }
            command => panic!("unexpected command: {command:?}"),
        }

        let fourth = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("destroy");
        match fourth.command {
            AppCommand::DestroyTray { reply } => {
                let _ = reply.send(Ok(()));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    });

    create_tray(TrayCommandConfig {
        tooltip: Some("Volt".to_string()),
        icon_rgba: None,
        icon_width: DEFAULT_TRAY_ICON_SIZE,
        icon_height: DEFAULT_TRAY_ICON_SIZE,
    })
    .expect("create tray");
    set_tray_tooltip("Volt".to_string()).expect("set tray tooltip");
    set_tray_visible(false).expect("set tray visible");
    destroy_tray().expect("destroy tray");

    shutdown_test_bridge(lifecycle);
    let _ = responder.join();
}
