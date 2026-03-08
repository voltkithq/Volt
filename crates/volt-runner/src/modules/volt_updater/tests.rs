use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use serde_json::json;
use volt_core::updater::UpdateInfo;

use super::config::{
    EMBEDDED_UPDATE_PUBLIC_KEY, UpdateCheckOptions, check_for_update_with_public_key,
    embedded_update_public_key, ensure_update_version_is_newer, ensure_updater_permissions,
    normalize_non_empty, parse_check_options_json, parse_update_info_json,
};
use super::events::{
    begin_update_install_operation, finish_update_install_operation, is_update_install_cancelled,
    mark_active_update_install_cancelled, reset_update_install_state_for_tests,
};
use std::sync::Mutex;

static UPDATE_INSTALL_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn normalize_non_empty_rejects_blank_values() {
    assert!(normalize_non_empty("".to_string(), "field").is_err());
    assert!(normalize_non_empty("   ".to_string(), "field").is_err());
    assert_eq!(
        normalize_non_empty("value".to_string(), "field").expect("normalized"),
        "value".to_string()
    );
}

#[test]
fn parse_check_options_json_validates_required_fields() {
    let parsed = parse_check_options_json(json!({
        "url": "https://updates.example.com/check",
        "currentVersion": "1.2.3"
    }))
    .expect("parse options");
    assert_eq!(parsed.url, "https://updates.example.com/check");
    assert_eq!(parsed.current_version, "1.2.3");

    assert!(
        parse_check_options_json(json!({
            "url": "",
            "currentVersion": "1.0.0"
        }))
        .is_err()
    );
    assert!(
        parse_check_options_json(json!({
            "url": "https://updates.example.com/check",
            "currentVersion": ""
        }))
        .is_err()
    );
}

#[test]
fn parse_update_info_json_requires_sha256() {
    let parsed = parse_update_info_json(json!({
        "version": "1.2.3",
        "url": "https://updates.example.com/app.exe",
        "signature": "c2ln",
        "sha256": "abcd"
    }))
    .expect("parse update info");
    assert_eq!(parsed.sha256, "abcd");

    assert!(
        parse_update_info_json(json!({
            "version": "1.2.3",
            "url": "https://updates.example.com/app.exe",
            "signature": "c2ln"
        }))
        .is_err()
    );
}

#[test]
fn embedded_update_public_key_is_reported_when_missing() {
    if EMBEDDED_UPDATE_PUBLIC_KEY.trim().is_empty() {
        assert!(embedded_update_public_key().is_err());
    } else {
        assert!(embedded_update_public_key().is_ok());
    }
}

#[test]
fn ensure_updater_permissions_requires_fs_permission() {
    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        permissions: vec![],
        ..Default::default()
    })
    .expect("configure module state");
    assert!(ensure_updater_permissions().is_err());

    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        permissions: vec!["fs".to_string(), "http".to_string()],
        ..Default::default()
    })
    .expect("configure module state");
    assert!(ensure_updater_permissions().is_ok());
}

fn check_for_update_against_single_response(
    status_line: &str,
    body: &str,
) -> Result<Option<UpdateInfo>, String> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buf = [0_u8; 2048];
        let _ = stream.read(&mut request_buf);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });

    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        permissions: vec!["fs".to_string(), "http".to_string()],
        ..Default::default()
    })
    .expect("configure module state");

    let result = check_for_update_with_public_key(
        UpdateCheckOptions {
            url: format!("http://{addr}/update"),
            current_version: "1.0.0".to_string(),
        },
        "unused-public-key-for-check".to_string(),
    );
    server.join().expect("join server");
    result
}

#[test]
fn check_for_update_returns_none_for_204() {
    let result = check_for_update_against_single_response("204 No Content", "")
        .expect("update check should succeed");
    assert!(result.is_none());
}

#[test]
fn ensure_update_version_is_newer_rejects_equal_or_older_versions() {
    let current_version = super::config::current_app_version();
    assert!(ensure_update_version_is_newer(&current_version).is_err());
    assert!(ensure_update_version_is_newer("0.0.1").is_err());
}

#[test]
fn update_install_cancel_flag_tracks_only_active_operation() {
    let _lock = UPDATE_INSTALL_TEST_LOCK
        .lock()
        .expect("update install test lock");
    reset_update_install_state_for_tests();
    let operation_id = begin_update_install_operation().expect("begin operation");
    assert!(!is_update_install_cancelled(operation_id));
    mark_active_update_install_cancelled();
    assert!(is_update_install_cancelled(operation_id));
    finish_update_install_operation(operation_id);
    reset_update_install_state_for_tests();
}

#[test]
fn begin_update_install_operation_rejects_concurrent_operation() {
    let _lock = UPDATE_INSTALL_TEST_LOCK
        .lock()
        .expect("update install test lock");
    reset_update_install_state_for_tests();
    let operation_id = begin_update_install_operation().expect("begin first operation");
    assert!(begin_update_install_operation().is_err());
    finish_update_install_operation(operation_id);
    reset_update_install_state_for_tests();
}
