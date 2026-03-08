use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;

use super::args::{InstallArgs, RollbackArgs};

#[cfg(target_os = "windows")]
pub(crate) fn verify_invoker_process_matches_target(
    pid: u32,
    target_path: &Path,
) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return Err(format!(
                "cannot validate process {pid}: process is not running or not accessible"
            ));
        }

        let mut len: u32 = 32_768;
        let mut buffer = vec![0_u16; len as usize];
        let status = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut len);
        CloseHandle(handle);

        if status == 0 {
            return Err(format!(
                "failed to resolve executable path for process {pid}"
            ));
        }

        buffer.truncate(len as usize);
        let process_path = PathBuf::from(OsString::from_wide(&buffer));
        let canonical_process = process_path.canonicalize().map_err(|error| {
            format!(
                "failed to canonicalize process executable '{}': {error}",
                process_path.display()
            )
        })?;
        if normalize_windows_path(&canonical_process) != normalize_windows_path(target_path) {
            return Err(format!(
                "refusing update: pid {pid} executable '{}' does not match target '{}'",
                canonical_process.display(),
                target_path.display()
            ));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub(crate) fn validate_install_paths(args: &InstallArgs) -> Result<(PathBuf, PathBuf), String> {
    let (helper_exe, helper_dir) = resolve_helper_context()?;
    let target_path = validate_target_path(&helper_exe, &helper_dir, &args.target_path)?;

    if !args.staged_path.is_absolute() {
        return Err("--staged must be an absolute path".to_string());
    }
    let staged_path = args.staged_path.canonicalize().map_err(|error| {
        format!(
            "failed to resolve staged payload '{}': {error}",
            args.staged_path.display()
        )
    })?;
    let temp_dir = std::env::temp_dir().canonicalize().map_err(|error| {
        format!(
            "failed to resolve temporary directory '{}': {error}",
            std::env::temp_dir().display()
        )
    })?;
    if !is_within_directory(&temp_dir, &staged_path) {
        return Err(format!(
            "staged payload '{}' must be within '{}'",
            staged_path.display(),
            temp_dir.display()
        ));
    }
    let staged_file_name = staged_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "staged payload '{}' must have a valid UTF-8 file name",
                staged_path.display()
            )
        })?;
    let staged_prefix = format!("volt-update-{}-", args.pid);
    if !staged_file_name.starts_with(&staged_prefix) || !staged_file_name.ends_with(".bin") {
        return Err(format!(
            "staged payload '{}' does not match expected secure pattern '{}*.bin'",
            staged_file_name, staged_prefix
        ));
    }

    Ok((target_path, staged_path))
}

#[cfg(target_os = "windows")]
pub(crate) fn validate_rollback_paths(
    args: &RollbackArgs,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let (helper_exe, helper_dir) = resolve_helper_context()?;
    let target_path = validate_target_path(&helper_exe, &helper_dir, &args.target_path)?;

    if !args.backup_path.is_absolute() {
        return Err("--backup must be an absolute path".to_string());
    }
    let backup_path = args.backup_path.canonicalize().map_err(|error| {
        format!(
            "failed to resolve backup payload '{}': {error}",
            args.backup_path.display()
        )
    })?;
    if !is_within_directory(&helper_dir, &backup_path) {
        return Err(format!(
            "backup payload '{}' must be inside helper directory '{}'",
            backup_path.display(),
            helper_dir.display()
        ));
    }
    let expected_backup_path = target_path.with_extension("old");
    if normalize_windows_path(&backup_path) != normalize_windows_path(&expected_backup_path) {
        return Err(format!(
            "backup payload '{}' must match expected rollback path '{}'",
            backup_path.display(),
            expected_backup_path.display()
        ));
    }

    if !args.pending_marker_path.is_absolute() {
        return Err("--pending-marker must be an absolute path".to_string());
    }
    let marker_path = args.pending_marker_path.canonicalize().map_err(|error| {
        format!(
            "failed to resolve pending marker '{}': {error}",
            args.pending_marker_path.display()
        )
    })?;
    if !is_within_directory(&helper_dir, &marker_path) {
        return Err(format!(
            "pending marker '{}' must be inside helper directory '{}'",
            marker_path.display(),
            helper_dir.display()
        ));
    }
    let marker_name = marker_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !marker_name.ends_with(".volt-update-pending.json") {
        return Err(format!(
            "pending marker '{}' must end with '.volt-update-pending.json'",
            marker_name
        ));
    }

    Ok((target_path, backup_path, marker_path))
}

#[cfg(target_os = "windows")]
fn resolve_helper_context() -> Result<(PathBuf, PathBuf), String> {
    let helper_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve updater helper executable path: {error}"))?;
    let helper_exe = helper_exe.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize updater helper path '{}': {error}",
            helper_exe.display()
        )
    })?;
    let helper_dir = helper_exe
        .parent()
        .ok_or_else(|| {
            format!(
                "updater helper executable '{}' has no parent directory",
                helper_exe.display()
            )
        })?
        .to_path_buf();
    Ok((helper_exe, helper_dir))
}

#[cfg(target_os = "windows")]
fn validate_target_path(
    helper_exe: &Path,
    helper_dir: &Path,
    target_arg: &Path,
) -> Result<PathBuf, String> {
    if !target_arg.is_absolute() {
        return Err("--target must be an absolute path".to_string());
    }
    let target_path = target_arg.canonicalize().map_err(|error| {
        format!(
            "failed to resolve target executable '{}': {error}",
            target_arg.display()
        )
    })?;
    if !is_within_directory(helper_dir, &target_path) {
        return Err(format!(
            "target executable '{}' must be inside helper directory '{}'",
            target_path.display(),
            helper_dir.display()
        ));
    }
    if normalize_windows_path(&target_path) == normalize_windows_path(helper_exe) {
        return Err("target executable must not point to volt-updater-helper.exe".to_string());
    }
    Ok(target_path)
}

#[cfg(target_os = "windows")]
pub(crate) fn wait_for_process_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, WaitForSingleObject,
    };
    const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

    unsafe {
        let handle = OpenProcess(
            SYNCHRONIZE_ACCESS | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        );
        if handle.is_null() {
            // Process already exited between pre-flight validation and wait.
            return Ok(());
        }

        let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
        let wait_result = WaitForSingleObject(handle, timeout_ms);
        CloseHandle(handle);

        match wait_result {
            WAIT_OBJECT_0 => Ok(()),
            WAIT_TIMEOUT => Err(format!(
                "timed out waiting for process {pid} to exit after {} seconds",
                timeout.as_secs()
            )),
            WAIT_FAILED => Err(format!(
                "failed while waiting for process {pid} to exit (WaitForSingleObject returned WAIT_FAILED)"
            )),
            code => Err(format!("unexpected wait result {code} for process {pid}")),
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn normalize_windows_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_start_matches("\\\\?\\")
        .to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn is_within_directory(base_dir: &Path, candidate: &Path) -> bool {
    let base = normalize_windows_path(base_dir);
    let candidate = normalize_windows_path(candidate);
    candidate == base || candidate.starts_with(&(base + "\\"))
}

#[cfg(test)]
mod tests;
