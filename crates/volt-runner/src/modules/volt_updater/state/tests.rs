use serde_json::json;
use std::fs;
use std::sync::Mutex;

use super::storage::{normalize_sha256_hex, now_unix_ms, read_json_file, write_json_atomic};
#[allow(unused_imports)]
use super::*;

static STATE_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn normalize_sha256_hex_accepts_lowercase_digest() {
    let digest = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let normalized = normalize_sha256_hex(digest, "sha256").expect("normalize digest");
    assert_eq!(normalized, digest);
}

#[test]
fn normalize_sha256_hex_rejects_invalid_digest() {
    assert!(normalize_sha256_hex("ABC", "sha256").is_err());
    assert!(
        normalize_sha256_hex(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            "sha256"
        )
        .is_err()
    );
}

#[test]
fn write_json_atomic_replaces_existing_file() {
    let _lock = STATE_TEST_LOCK.lock().expect("state test lock");
    let temp_root = std::env::temp_dir().join(format!(
        "volt_updater_state_test_{}_{}",
        std::process::id(),
        now_unix_ms().unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root).expect("create temp root");
    let marker_path = temp_root.join("marker.json");

    write_json_atomic(&marker_path, &json!({ "version": 1 })).expect("write initial marker");
    write_json_atomic(&marker_path, &json!({ "version": 2 })).expect("replace marker");

    let parsed: serde_json::Value = read_json_file(&marker_path).expect("read marker");
    assert_eq!(parsed["version"], 2);

    let _ = fs::remove_dir_all(&temp_root);
}

#[cfg(target_os = "windows")]
#[test]
fn prepare_startup_recovery_marks_first_launch_attempt() {
    let _lock = STATE_TEST_LOCK.lock().expect("state test lock");
    let current_exe = std::env::current_exe()
        .expect("current exe")
        .canonicalize()
        .expect("canonical current exe");
    let marker_path = pending_marker_path_for_target(&current_exe).expect("pending marker path");
    let success_marker_path =
        success_marker_path_for_target(&current_exe).expect("success marker path");
    let rollback_path = current_exe.with_extension("old");
    let _ = fs::remove_file(&marker_path);
    let _ = fs::remove_file(&success_marker_path);

    let digest = "a".repeat(64);
    let marker = PendingUpdateMarker {
        schema_version: MARKER_SCHEMA_VERSION,
        target_path: current_exe.to_string_lossy().to_string(),
        rollback_path: rollback_path.to_string_lossy().to_string(),
        expected_target_sha256: digest.clone(),
        rollback_sha256: digest,
        previous_version: "1.0.0".to_string(),
        target_version: "1.0.1".to_string(),
        startup_attempts: 0,
        created_at_unix_ms: now_unix_ms().expect("timestamp"),
        first_launch_started_at_unix_ms: None,
    };
    write_json_atomic(&marker_path, &marker).expect("write marker");

    let window = prepare_startup_recovery()
        .expect("prepare startup recovery")
        .expect("healthy startup window");
    assert_eq!(window.marker_path, marker_path);
    assert_eq!(window.rollback_path, rollback_path);

    let updated: PendingUpdateMarker = read_json_file(&marker_path).expect("updated marker");
    assert_eq!(updated.startup_attempts, 1);
    assert!(updated.first_launch_started_at_unix_ms.is_some());

    remove_file_with_warning(&marker_path, "test pending-update marker");
    remove_file_with_warning(&success_marker_path, "test success marker");
}
