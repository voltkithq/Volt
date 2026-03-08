// Most consumers are Windows-only; suppress dead_code on Linux.
#![allow(dead_code)]

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(super) fn write_json_atomic<T: Serialize>(path: &Path, payload: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(payload)
        .map_err(|error| format!("failed to serialize JSON: {error}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("path '{}' has no valid UTF-8 file name", path.display()))?;
    let temp_path: PathBuf = path.with_file_name(format!("{file_name}.tmp"));
    fs::write(&temp_path, bytes).map_err(|error| {
        format!(
            "failed to write temp file '{}': {error}",
            temp_path.display()
        )
    })?;
    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "failed to move temp file '{}' to '{}': {error}",
            temp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

pub(super) fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse JSON from '{}': {error}", path.display()))
}

pub(super) fn remove_file_with_warning(path: &Path, label: &str) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            error = %error,
            label = label,
            path = %path.display(),
            "failed to remove updater file"
        );
    }
}

pub(super) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("failed to open '{}': {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn normalize_sha256_hex(input: &str, field_name: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.len() != 64 {
        return Err(format!(
            "{field_name} must be a 64-character lowercase hex digest"
        ));
    }
    if !trimmed
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{field_name} must contain only lowercase hex characters"
        ));
    }
    Ok(trimmed.to_string())
}

pub(super) fn now_unix_ms() -> Result<u64, String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?;
    Ok(duration.as_millis() as u64)
}
