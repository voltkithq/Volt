use std::path::{Path, PathBuf};

use semver::Version;

use super::PendingUpdateMarker;
use super::storage::sha256_file;
use super::{PENDING_MARKER_SUFFIX, SUCCESS_MARKER_SUFFIX};

const UPDATE_HELPER_WAIT_TIMEOUT_SECS: u64 = 600;
const UPDATE_HELPER_BINARY_NAME: &str = "volt-updater-helper.exe";

pub(super) fn validate_rollback_candidate(
    marker: &PendingUpdateMarker,
    current_exe: &Path,
) -> Result<(), String> {
    let previous = Version::parse(&marker.previous_version).map_err(|error| {
        format!(
            "invalid rollback previous version '{}': {error}",
            marker.previous_version
        )
    })?;
    let target = Version::parse(&marker.target_version).map_err(|error| {
        format!(
            "invalid rollback target version '{}': {error}",
            marker.target_version
        )
    })?;
    if previous >= target {
        return Err(format!(
            "rollback rejected: previous version {} must be less than target version {}",
            marker.previous_version, marker.target_version
        ));
    }

    let current = runner_app_version()
        .ok()
        .and_then(|value| Version::parse(&value).ok());
    if let Some(current) = current
        && current < previous
    {
        return Err(format!(
            "rollback rejected: running version {} is older than rollback candidate {}",
            current, previous
        ));
    }

    let rollback_path = PathBuf::from(&marker.rollback_path);
    if !rollback_path.exists() {
        return Err(format!(
            "rollback candidate '{}' does not exist",
            rollback_path.display()
        ));
    }
    let rollback_hash = sha256_file(&rollback_path)?;
    if rollback_hash != marker.rollback_sha256 {
        return Err(format!(
            "rollback candidate checksum mismatch: expected {}, got {}",
            marker.rollback_sha256, rollback_hash
        ));
    }

    let marker_target = PathBuf::from(&marker.target_path);
    if normalize_windows_path(&marker_target)? != normalize_windows_path(current_exe)? {
        return Err(format!(
            "rollback marker target '{}' does not match current executable '{}'",
            marker.target_path,
            current_exe.display()
        ));
    }

    Ok(())
}

pub(super) fn spawn_rollback_helper(
    marker: &PendingUpdateMarker,
    marker_path: &Path,
) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable path: {error}"))?;
    let helper_path = current_exe
        .parent()
        .ok_or_else(|| {
            format!(
                "current executable '{}' has no parent directory",
                current_exe.display()
            )
        })?
        .join(UPDATE_HELPER_BINARY_NAME);
    if !helper_path.exists() {
        return Err(format!(
            "rollback helper not found at '{}'",
            helper_path.display()
        ));
    }

    std::process::Command::new(&helper_path)
        .arg("--mode")
        .arg("rollback")
        .arg("--pid")
        .arg(std::process::id().to_string())
        .arg("--target")
        .arg(&marker.target_path)
        .arg("--backup")
        .arg(&marker.rollback_path)
        .arg("--backup-sha256")
        .arg(&marker.rollback_sha256)
        .arg("--pending-marker")
        .arg(marker_path)
        .arg("--wait-timeout-secs")
        .arg(UPDATE_HELPER_WAIT_TIMEOUT_SECS.to_string())
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            format!(
                "failed to launch rollback helper '{}': {error}",
                helper_path.display()
            )
        })
}

pub(super) fn pending_marker_path_for_target(target_path: &Path) -> Result<PathBuf, String> {
    let file_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "target path '{}' must have a UTF-8 file name",
                target_path.display()
            )
        })?;
    Ok(target_path.with_file_name(format!("{file_name}{PENDING_MARKER_SUFFIX}")))
}

pub(super) fn success_marker_path_for_target(target_path: &Path) -> Result<PathBuf, String> {
    let file_name = target_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "target path '{}' must have a UTF-8 file name",
                target_path.display()
            )
        })?;
    Ok(target_path.with_file_name(format!("{file_name}{SUCCESS_MARKER_SUFFIX}")))
}

pub(super) fn normalize_windows_path(path: &Path) -> Result<String, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize path '{}': {error}", path.display()))?;
    Ok(canonical
        .to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .to_ascii_lowercase())
}

fn runner_app_version() -> Result<String, String> {
    let value = option_env!("VOLT_APP_VERSION")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .trim()
        .to_string();
    if value.is_empty() {
        return Err("runner app version is empty".to_string());
    }
    Ok(value)
}
