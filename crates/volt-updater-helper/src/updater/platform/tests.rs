use super::*;
#[cfg(target_os = "windows")]
use crate::updater::args::{InstallArgs, RollbackArgs};
#[cfg(target_os = "windows")]
use std::fs;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
#[test]
fn validate_install_paths_accepts_expected_windows_layout() {
    let pid = std::process::id();
    let (target_path, staged_path) = create_valid_test_paths(pid);
    let args = install_args_for_paths(pid, target_path.clone(), staged_path.clone());

    let validated = validate_install_paths(&args).expect("validate install paths");
    assert_eq!(
        normalize_windows_path(&validated.0),
        normalize_windows_path(&target_path.canonicalize().expect("canonical target"))
    );
    assert_eq!(
        normalize_windows_path(&validated.1),
        normalize_windows_path(&staged_path.canonicalize().expect("canonical staged"))
    );

    cleanup_test_paths(&target_path, &staged_path);
}

#[cfg(target_os = "windows")]
#[test]
fn validate_install_paths_rejects_staged_payload_outside_temp_directory() {
    let pid = std::process::id();
    let (target_path, staged_path) = create_valid_test_paths(pid);

    let helper_dir = std::env::current_exe()
        .expect("current exe")
        .canonicalize()
        .expect("canonical helper exe")
        .parent()
        .expect("helper parent")
        .to_path_buf();
    let outside_staged = helper_dir.join(format!("volt-update-{pid}-outside.bin"));
    fs::write(&outside_staged, b"payload").expect("write outside staged file");

    let args = install_args_for_paths(pid, target_path.clone(), outside_staged.clone());
    let error = validate_install_paths(&args).expect_err("validate should fail");
    assert!(error.contains("must be within"));

    cleanup_test_paths(&target_path, &staged_path);
    let _ = fs::remove_file(outside_staged);
}

#[cfg(target_os = "windows")]
#[test]
fn validate_rollback_paths_accepts_expected_layout() {
    let pid = std::process::id();
    let (target_path, staged_path) = create_valid_test_paths(pid);
    let backup_path = target_path.with_extension("old");
    fs::write(&backup_path, b"backup").expect("write backup file");
    let marker_path = target_path.with_file_name(format!(
        "{}.volt-update-pending.json",
        target_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("target filename")
    ));
    fs::write(&marker_path, b"{}").expect("write marker file");

    let args = RollbackArgs {
        pid,
        target_path: target_path.clone(),
        backup_path: backup_path.clone(),
        backup_sha256: "a".repeat(64),
        pending_marker_path: marker_path.clone(),
        wait_timeout_secs: 600,
    };

    let validated = validate_rollback_paths(&args).expect("validate rollback paths");
    assert_eq!(
        normalize_windows_path(&validated.0),
        normalize_windows_path(&target_path.canonicalize().expect("canonical target"))
    );
    assert_eq!(
        normalize_windows_path(&validated.1),
        normalize_windows_path(&backup_path.canonicalize().expect("canonical backup"))
    );
    assert_eq!(
        normalize_windows_path(&validated.2),
        normalize_windows_path(&marker_path.canonicalize().expect("canonical marker"))
    );

    cleanup_test_paths(&target_path, &staged_path);
    let _ = fs::remove_file(backup_path);
    let _ = fs::remove_file(marker_path);
}

#[cfg(target_os = "windows")]
fn install_args_for_paths(pid: u32, target_path: PathBuf, staged_path: PathBuf) -> InstallArgs {
    InstallArgs {
        pid,
        target_path,
        staged_path,
        expected_sha256: "a".repeat(64),
        wait_timeout_secs: 600,
    }
}

#[cfg(target_os = "windows")]
fn create_valid_test_paths(pid: u32) -> (PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    let helper_dir = std::env::current_exe()
        .expect("current exe")
        .canonicalize()
        .expect("canonical helper exe")
        .parent()
        .expect("helper parent")
        .to_path_buf();

    let target_path = helper_dir.join(format!("volt-updater-target-{pid}-{nonce}.exe"));
    fs::write(&target_path, b"target").expect("write target file");

    let staged_path = std::env::temp_dir().join(format!("volt-update-{pid}-{nonce}.bin"));
    fs::write(&staged_path, b"payload").expect("write staged file");

    (target_path, staged_path)
}

#[cfg(target_os = "windows")]
fn cleanup_test_paths(target_path: &Path, staged_path: &Path) {
    let _ = fs::remove_file(target_path);
    let _ = fs::remove_file(staged_path);
}
