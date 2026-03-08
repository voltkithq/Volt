use super::util::sha256_hex;
use super::verification::UpdateError;

#[cfg(target_os = "windows")]
const ENV_UPDATE_HELPER_PATH: &str = "VOLT_UPDATE_HELPER_PATH";
#[cfg(target_os = "windows")]
const UPDATE_HELPER_BINARY_NAME: &str = "volt-updater-helper.exe";
#[cfg(target_os = "windows")]
const UPDATE_HELPER_WAIT_TIMEOUT_SECS: u64 = 600;

/// Get the current platform target string.
pub(super) fn current_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "darwin-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "darwin-arm64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    {
        "unknown"
    }
}

#[cfg(target_os = "windows")]
pub(super) fn apply_update(data: &[u8]) -> Result<(), UpdateError> {
    let current_exe = std::env::current_exe()
        .map_err(|e| UpdateError::ApplyFailed(format!("cannot determine current exe: {e}")))?;
    let helper_path = resolve_update_helper_path(&current_exe)?;
    let expected_sha256 = sha256_hex(data);
    let staged_payload = stage_update_payload(data)?;

    if let Err(error) = spawn_update_helper(
        &helper_path,
        &current_exe,
        &staged_payload,
        &expected_sha256,
    ) {
        let _ = std::fs::remove_file(&staged_payload);
        return Err(error);
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(super) fn apply_update(data: &[u8]) -> Result<(), UpdateError> {
    use std::io::Write;

    let current_exe = std::env::current_exe()
        .map_err(|e| UpdateError::ApplyFailed(format!("cannot determine current exe: {e}")))?;

    let temp_path = current_exe.with_extension("update");
    let _ = std::fs::remove_file(&temp_path);

    let mut temp_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|e| UpdateError::ApplyFailed(format!("failed to create temp file: {e}")))?;
    temp_file
        .write_all(data)
        .map_err(|e| UpdateError::ApplyFailed(format!("failed to write temp file: {e}")))?;
    temp_file
        .sync_all()
        .map_err(|e| UpdateError::ApplyFailed(format!("failed to flush temp file: {e}")))?;
    drop(temp_file);

    let expected_hash = sha256_hex(data);
    let written = std::fs::read(&temp_path).map_err(|e| {
        UpdateError::ApplyFailed(format!("failed to read temp file for verify: {e}"))
    })?;
    let actual_hash = sha256_hex(&written);
    if actual_hash != expected_hash {
        let _ = std::fs::remove_file(&temp_path);
        return Err(UpdateError::ApplyFailed(
            "temporary update file failed post-write SHA-256 verification".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&temp_path, perms)
            .map_err(|e| UpdateError::ApplyFailed(format!("failed to set permissions: {e}")))?;
    }

    std::fs::rename(&temp_path, &current_exe)
        .map_err(|e| UpdateError::ApplyFailed(format!("failed to replace binary: {e}")))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn resolve_update_helper_path(
    current_exe: &std::path::Path,
) -> Result<std::path::PathBuf, UpdateError> {
    let current_dir = current_exe.parent().ok_or_else(|| {
        UpdateError::ApplyFailed("failed to determine executable directory".to_string())
    })?;

    if let Ok(configured) = std::env::var(ENV_UPDATE_HELPER_PATH) {
        let trimmed = configured.trim();
        if !trimmed.is_empty() {
            return validate_update_helper_override_path(current_dir, trimmed);
        }
    }

    Ok(current_dir.join(UPDATE_HELPER_BINARY_NAME))
}

#[cfg(target_os = "windows")]
fn validate_update_helper_override_path(
    current_dir: &std::path::Path,
    configured: &str,
) -> Result<std::path::PathBuf, UpdateError> {
    let configured_path = std::path::PathBuf::from(configured);
    if !configured_path.is_absolute() {
        return Err(UpdateError::ApplyFailed(format!(
            "{ENV_UPDATE_HELPER_PATH} must be an absolute path"
        )));
    }

    let configured_name = configured_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !configured_name.eq_ignore_ascii_case(UPDATE_HELPER_BINARY_NAME) {
        return Err(UpdateError::ApplyFailed(format!(
            "{ENV_UPDATE_HELPER_PATH} must point to '{}'",
            UPDATE_HELPER_BINARY_NAME
        )));
    }

    let canonical_current_dir = current_dir.canonicalize().map_err(|error| {
        UpdateError::ApplyFailed(format!(
            "failed to resolve executable directory '{}': {error}",
            current_dir.display()
        ))
    })?;
    let canonical_configured = configured_path.canonicalize().map_err(|error| {
        UpdateError::ApplyFailed(format!(
            "failed to resolve update helper path '{}': {error}",
            configured_path.display()
        ))
    })?;
    let configured_parent = canonical_configured.parent().ok_or_else(|| {
        UpdateError::ApplyFailed(format!(
            "configured update helper path '{}' has no parent directory",
            canonical_configured.display()
        ))
    })?;
    if normalize_windows_path(configured_parent) != normalize_windows_path(&canonical_current_dir) {
        return Err(UpdateError::ApplyFailed(format!(
            "{ENV_UPDATE_HELPER_PATH} must point to '{}' inside '{}'",
            UPDATE_HELPER_BINARY_NAME,
            canonical_current_dir.display()
        )));
    }

    Ok(canonical_configured)
}

#[cfg(target_os = "windows")]
fn normalize_windows_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn stage_update_payload(data: &[u8]) -> Result<std::path::PathBuf, UpdateError> {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| UpdateError::ApplyFailed(format!("system clock error: {error}")))?
        .as_nanos();
    let staged_payload =
        std::env::temp_dir().join(format!("volt-update-{}-{nonce}.bin", std::process::id()));

    let mut staged_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged_payload)
        .map_err(|error| {
            UpdateError::ApplyFailed(format!(
                "failed to create staged update payload '{}': {error}",
                staged_payload.display()
            ))
        })?;

    staged_file.write_all(data).map_err(|error| {
        UpdateError::ApplyFailed(format!("failed to write staged payload: {error}"))
    })?;
    staged_file.sync_all().map_err(|error| {
        UpdateError::ApplyFailed(format!("failed to flush staged payload: {error}"))
    })?;
    drop(staged_file);

    Ok(staged_payload)
}

#[cfg(target_os = "windows")]
fn spawn_update_helper(
    helper_path: &std::path::Path,
    current_exe: &std::path::Path,
    staged_payload: &std::path::Path,
    expected_sha256: &str,
) -> Result<(), UpdateError> {
    if !helper_path.exists() {
        return Err(UpdateError::ApplyFailed(format!(
            "update helper not found at '{}'. Ensure '{}' is packaged with the app.",
            helper_path.display(),
            UPDATE_HELPER_BINARY_NAME
        )));
    }

    std::process::Command::new(helper_path)
        .arg("--pid")
        .arg(std::process::id().to_string())
        .arg("--target")
        .arg(current_exe)
        .arg("--staged")
        .arg(staged_payload)
        .arg("--sha256")
        .arg(expected_sha256)
        .arg("--wait-timeout-secs")
        .arg(UPDATE_HELPER_WAIT_TIMEOUT_SECS.to_string())
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            UpdateError::ApplyFailed(format!(
                "failed to launch update helper '{}': {error}",
                helper_path.display()
            ))
        })
}
