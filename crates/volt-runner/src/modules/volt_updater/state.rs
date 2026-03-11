// Most of this module is Windows-only; suppress dead_code on Linux.
#![allow(dead_code, unused_imports)]

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use self::storage::{
    normalize_sha256_hex, now_unix_ms, read_json_file, remove_file_with_warning, sha256_file,
    write_json_atomic,
};
#[cfg(target_os = "windows")]
use self::windows::{
    pending_marker_path_for_target, spawn_rollback_helper, success_marker_path_for_target,
    validate_rollback_candidate,
};

#[path = "state/storage.rs"]
mod storage;
#[cfg(test)]
#[path = "state/tests.rs"]
mod tests;
#[cfg(target_os = "windows")]
#[path = "state/windows.rs"]
mod windows;

const HEALTHY_STARTUP_WINDOW_SECS: u64 = 30;
const PENDING_MARKER_SUFFIX: &str = ".volt-update-pending.json";
const SUCCESS_MARKER_SUFFIX: &str = ".volt-update-success.json";
const MARKER_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingUpdateMarker {
    schema_version: u8,
    target_path: String,
    rollback_path: String,
    expected_target_sha256: String,
    rollback_sha256: String,
    previous_version: String,
    target_version: String,
    startup_attempts: u32,
    created_at_unix_ms: u64,
    first_launch_started_at_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateSuccessMarker {
    schema_version: u8,
    target_version: String,
    previous_version: String,
    completed_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct HealthyStartupWindow {
    marker_path: PathBuf,
    rollback_path: PathBuf,
    success_marker_path: PathBuf,
    target_version: String,
    previous_version: String,
}

pub(crate) fn persist_pending_update_marker(
    target_version: &str,
    previous_version: &str,
    expected_target_sha256: &str,
) -> Result<Option<PathBuf>, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = target_version;
        let _ = previous_version;
        let _ = expected_target_sha256;
        Ok(None)
    }

    #[cfg(target_os = "windows")]
    {
        let target_path = std::env::current_exe()
            .map_err(|error| format!("failed to resolve current executable path: {error}"))?
            .canonicalize()
            .map_err(|error| format!("failed to canonicalize current executable path: {error}"))?;
        let rollback_path = target_path.with_extension("old");
        let marker_path = pending_marker_path_for_target(&target_path)?;
        let rollback_sha256 = sha256_file(&target_path)?;
        let marker = PendingUpdateMarker {
            schema_version: MARKER_SCHEMA_VERSION,
            target_path: target_path.to_string_lossy().to_string(),
            rollback_path: rollback_path.to_string_lossy().to_string(),
            expected_target_sha256: normalize_sha256_hex(expected_target_sha256, "sha256")?,
            rollback_sha256,
            previous_version: previous_version.to_string(),
            target_version: target_version.to_string(),
            startup_attempts: 0,
            created_at_unix_ms: now_unix_ms()?,
            first_launch_started_at_unix_ms: None,
        };

        write_json_atomic(&marker_path, &marker)?;
        Ok(Some(marker_path))
    }
}

pub(crate) fn remove_pending_update_marker(path: &Path) {
    remove_file_with_warning(path, "pending-update marker");
}

pub(crate) fn prepare_startup_recovery() -> Result<Option<HealthyStartupWindow>, String> {
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }

    #[cfg(target_os = "windows")]
    {
        let current_exe = std::env::current_exe()
            .map_err(|error| format!("failed to resolve current executable path: {error}"))?
            .canonicalize()
            .map_err(|error| format!("failed to canonicalize current executable path: {error}"))?;
        let marker_path = pending_marker_path_for_target(&current_exe)?;
        if !marker_path.exists() {
            return Ok(None);
        }

        let mut marker: PendingUpdateMarker = match read_json_file(&marker_path) {
            Ok(value) => value,
            Err(error) => {
                remove_file_with_warning(&marker_path, "invalid pending-update marker");
                tracing::warn!("pending-update marker was invalid and has been removed: {error}");
                return Ok(None);
            }
        };

        if marker.schema_version != MARKER_SCHEMA_VERSION {
            remove_file_with_warning(&marker_path, "unsupported pending-update marker");
            tracing::warn!(
                "pending-update marker schema mismatch (expected {}, got {}) — removed",
                MARKER_SCHEMA_VERSION, marker.schema_version
            );
            return Ok(None);
        }

        let marker_target_path = PathBuf::from(&marker.target_path);
        if windows::normalize_windows_path(&marker_target_path)?
            != windows::normalize_windows_path(&current_exe)?
        {
            remove_file_with_warning(&marker_path, "stale pending-update marker");
            tracing::warn!(
                "pending-update marker target '{}' does not match current executable '{}' — removed",
                marker.target_path,
                current_exe.display()
            );
            return Ok(None);
        }

        if marker.startup_attempts > 0 {
            validate_rollback_candidate(&marker, &current_exe)?;
            spawn_rollback_helper(&marker, &marker_path)?;
            return Err(
                "detected interrupted startup after update; rollback has been scheduled. Relaunch the app once rollback completes.".to_string(),
            );
        }

        marker.startup_attempts = marker.startup_attempts.saturating_add(1);
        marker.first_launch_started_at_unix_ms = Some(now_unix_ms()?);
        write_json_atomic(&marker_path, &marker)?;

        Ok(Some(HealthyStartupWindow {
            marker_path,
            rollback_path: PathBuf::from(&marker.rollback_path),
            success_marker_path: success_marker_path_for_target(&current_exe)?,
            target_version: marker.target_version,
            previous_version: marker.previous_version,
        }))
    }
}

pub(crate) fn spawn_healthy_startup_clearer(window: HealthyStartupWindow) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = thread::Builder::new()
            .name("volt-update-health-window".to_string())
            .spawn(move || {
                thread::sleep(Duration::from_secs(HEALTHY_STARTUP_WINDOW_SECS));
                if let Err(error) = finalize_healthy_startup(window) {
                    tracing::warn!(
                        error = %error,
                        "updater startup-health finalization failed"
                    );
                }
            });
    }
}

#[cfg(target_os = "windows")]
fn finalize_healthy_startup(window: HealthyStartupWindow) -> Result<(), String> {
    if !window.marker_path.exists() {
        return Ok(());
    }

    remove_file_with_warning(&window.marker_path, "pending-update marker");
    remove_file_with_warning(&window.rollback_path, "rollback candidate");
    let success_marker = UpdateSuccessMarker {
        schema_version: MARKER_SCHEMA_VERSION,
        target_version: window.target_version,
        previous_version: window.previous_version,
        completed_at_unix_ms: now_unix_ms()?,
    };
    write_json_atomic(&window.success_marker_path, &success_marker)?;
    Ok(())
}
