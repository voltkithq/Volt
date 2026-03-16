mod resolve;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::UNIX_EPOCH;
use thiserror::Error;

pub use resolve::{safe_resolve, safe_resolve_for_create};
use resolve::{canonical_base, ensure_not_symlink, ensure_scoped_directory, verify_opened_path};

#[derive(Error, Debug)]
pub enum FsError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path security violation: {0}")]
    Security(String),

    #[error("path is outside the allowed scope")]
    OutOfScope,
}

/// File metadata info returned by stat().
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub readonly: bool,
    /// Last modification time as milliseconds since Unix epoch.
    pub modified_ms: f64,
    /// Creation time as milliseconds since Unix epoch.
    /// `None` on platforms/filesystems that do not support birth time.
    pub created_ms: Option<f64>,
}

/// Read a file's contents as bytes.
/// Post-open verification ensures no TOCTOU symlink swap can escape the sandbox.
pub fn read_file(base: &Path, path: &str) -> Result<Vec<u8>, FsError> {
    let resolved = safe_resolve(base, path)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved, &cb)?;
    let data = fs::read(&resolved)?;
    verify_opened_path(&resolved, &cb)?;
    Ok(data)
}

/// Read a file's contents as a UTF-8 string.
/// Post-open verification ensures no TOCTOU symlink swap can escape the sandbox.
pub fn read_file_text(base: &Path, path: &str) -> Result<String, FsError> {
    let resolved = safe_resolve(base, path)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved, &cb)?;
    let data = fs::read_to_string(&resolved)?;
    verify_opened_path(&resolved, &cb)?;
    Ok(data)
}

/// Write data to a file, creating it if it doesn't exist.
/// For existing files, a post-open symlink check prevents TOCTOU escapes.
/// For new files, `create_new(true)` fails if a symlink appears at the path.
pub fn write_file(base: &Path, path: &str, data: &[u8]) -> Result<(), FsError> {
    let resolved = safe_resolve_for_create(base, path)?;
    let cb = canonical_base(base)?;

    if resolved.exists() {
        ensure_not_symlink(&resolved)?;
        verify_opened_path(&resolved, &cb)?;
        fs::write(&resolved, data)?;
        verify_opened_path(&resolved, &cb)?;
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&resolved)?;
    file.write_all(data)?;
    Ok(())
}

/// List entries in a directory.
pub fn read_dir(base: &Path, path: &str) -> Result<Vec<String>, FsError> {
    let resolved = safe_resolve(base, path)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved, &cb)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(resolved)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            entries.push(name.to_string());
        }
    }
    Ok(entries)
}

/// Get metadata for a path.
pub fn stat(base: &Path, path: &str) -> Result<FileInfo, FsError> {
    let resolved = safe_resolve(base, path)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved, &cb)?;
    let meta = fs::metadata(resolved)?;

    let modified_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);

    let created_ms = meta
        .created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0);

    Ok(FileInfo {
        size: meta.len(),
        is_file: meta.is_file(),
        is_dir: meta.is_dir(),
        readonly: meta.permissions().readonly(),
        modified_ms,
        created_ms,
    })
}

/// Check whether a path exists within the scoped base directory.
pub fn exists(base: &Path, path: &str) -> Result<bool, FsError> {
    let resolved = safe_resolve(base, path)?;
    if !resolved.exists() {
        return Ok(false);
    }
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved, &cb)?;
    Ok(true)
}

/// Create a directory (and parents if needed).
pub fn mkdir(base: &Path, path: &str) -> Result<(), FsError> {
    let resolved = safe_resolve(base, path)?;
    ensure_scoped_directory(base, &resolved)
}

/// Remove a file or directory.
/// If the path is a directory, removal is recursive.
/// Double symlink check (before and after `is_dir`) narrows the TOCTOU window.
pub fn remove(base: &Path, path: &str) -> Result<(), FsError> {
    let resolved = safe_resolve(base, path)?;
    ensure_not_symlink(&resolved)?;
    let cb = canonical_base(base)?;
    if resolved == cb {
        return Err(FsError::Security(
            "Refusing to remove the base directory".to_string(),
        ));
    }

    if resolved.is_dir() {
        ensure_not_symlink(&resolved)?;
        verify_opened_path(&resolved, &cb)?;
        Ok(fs::remove_dir_all(resolved)?)
    } else {
        ensure_not_symlink(&resolved)?;
        verify_opened_path(&resolved, &cb)?;
        Ok(fs::remove_file(resolved)?)
    }
}

/// Rename (move) a file or directory within the scope.
pub fn rename(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    let resolved_from = safe_resolve(base, from)?;
    let resolved_to = safe_resolve_for_create(base, to)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved_from, &cb)?;

    if !resolved_from.exists() {
        return Err(FsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path does not exist: {from}"),
        )));
    }

    if resolved_to.exists() {
        return Err(FsError::Security(format!(
            "FS_ALREADY_EXISTS: destination already exists: {to}"
        )));
    }

    fs::rename(resolved_from, resolved_to)?;
    Ok(())
}

/// Copy a file within the scope.
/// Only files can be copied; use mkdir + recursive copy for directories.
pub fn copy(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    let resolved_from = safe_resolve(base, from)?;
    let resolved_to = safe_resolve_for_create(base, to)?;
    let cb = canonical_base(base)?;
    verify_opened_path(&resolved_from, &cb)?;

    if !resolved_from.exists() {
        return Err(FsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path does not exist: {from}"),
        )));
    }

    if !resolved_from.is_file() {
        return Err(FsError::Security(
            "copy only supports files, not directories".to_string(),
        ));
    }

    if resolved_to.exists() {
        return Err(FsError::Security(format!(
            "FS_ALREADY_EXISTS: destination already exists: {to}"
        )));
    }

    fs::copy(resolved_from, resolved_to)?;
    Ok(())
}

#[cfg(test)]
mod tests;
