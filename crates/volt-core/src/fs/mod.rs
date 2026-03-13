use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use thiserror::Error;

use crate::security::validate_path;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path security violation: {0}")]
    Security(String),

    #[error("path is outside the allowed scope")]
    OutOfScope,
}

/// Safely resolve a user-provided relative path against a base directory.
/// Rejects absolute paths, path traversal, and ensures the result is under base.
pub fn safe_resolve(base: &Path, user_path: &str) -> Result<PathBuf, FsError> {
    // Validate the path for traversal attacks and reserved names
    validate_path(user_path).map_err(FsError::Security)?;

    let resolved = base.join(user_path);

    // Canonicalize both paths and verify the resolved path is under base.
    // If the file doesn't exist yet, canonicalize the parent.
    let canonical_base = base
        .canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))?;

    // Try to canonicalize the full path first (works if file exists)
    let canonical_resolved = if resolved.exists() {
        resolved.canonicalize()?
    } else {
        // If the file doesn't exist, canonicalize the parent directory
        let parent = resolved
            .parent()
            .ok_or_else(|| FsError::Security("Cannot resolve parent directory".to_string()))?;

        if !parent.exists() {
            // Parent also doesn't exist - walk up to find nearest existing ancestor
            // Find the nearest existing ancestor and canonicalize from there
            let mut ancestor = parent.to_path_buf();
            let mut trailing_components = Vec::new();
            while !ancestor.exists() {
                if let Some(name) = ancestor.file_name() {
                    trailing_components.push(name.to_os_string());
                } else {
                    return Err(FsError::Security(
                        "Cannot resolve path ancestor".to_string(),
                    ));
                }
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| FsError::Security("Cannot resolve path ancestor".to_string()))?
                    .to_path_buf();
            }
            let mut canonical = ancestor.canonicalize()?;
            for component in trailing_components.into_iter().rev() {
                canonical.push(component);
            }
            if let Some(file_name) = resolved.file_name() {
                canonical.push(file_name);
            }
            canonical
        } else {
            let canonical_parent = parent.canonicalize()?;
            let file_name = resolved
                .file_name()
                .ok_or_else(|| FsError::Security("Invalid file name".to_string()))?;
            canonical_parent.join(file_name)
        }
    };

    // Verify the resolved path starts with the base
    if !canonical_resolved.starts_with(&canonical_base) {
        return Err(FsError::OutOfScope);
    }

    Ok(canonical_resolved)
}

/// Resolve a path for create/write flows while securely materializing any
/// missing parent directories inside the scoped base directory.
pub fn safe_resolve_for_create(base: &Path, user_path: &str) -> Result<PathBuf, FsError> {
    let resolved = safe_resolve(base, user_path)?;
    ensure_scoped_parent_dirs(base, &resolved)?;
    ensure_not_symlink(&resolved)?;
    Ok(resolved)
}

/// Read a file's contents as bytes.
pub fn read_file(base: &Path, path: &str) -> Result<Vec<u8>, FsError> {
    let resolved = safe_resolve(base, path)?;
    Ok(fs::read(resolved)?)
}

/// Read a file's contents as a UTF-8 string.
pub fn read_file_text(base: &Path, path: &str) -> Result<String, FsError> {
    let resolved = safe_resolve(base, path)?;
    Ok(fs::read_to_string(resolved)?)
}

/// Write data to a file, creating it if it doesn't exist.
pub fn write_file(base: &Path, path: &str, data: &[u8]) -> Result<(), FsError> {
    let resolved = safe_resolve_for_create(base, path)?;

    if resolved.exists() {
        fs::write(resolved, data)?;
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(resolved)?;
    file.write_all(data)?;
    Ok(())
}

/// List entries in a directory.
pub fn read_dir(base: &Path, path: &str) -> Result<Vec<String>, FsError> {
    let resolved = safe_resolve(base, path)?;
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
    Ok(resolved.exists())
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

/// Create a directory (and parents if needed).
pub fn mkdir(base: &Path, path: &str) -> Result<(), FsError> {
    let resolved = safe_resolve(base, path)?;
    ensure_scoped_directory(base, &resolved)
}

/// Remove a file or directory.
/// If the path is a directory, removal is recursive.
pub fn remove(base: &Path, path: &str) -> Result<(), FsError> {
    let resolved = safe_resolve(base, path)?;
    ensure_not_symlink(&resolved)?;
    let canonical_base = base
        .canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))?;
    if resolved == canonical_base {
        return Err(FsError::Security(
            "Refusing to remove the base directory".to_string(),
        ));
    }

    if resolved.is_dir() {
        Ok(fs::remove_dir_all(resolved)?)
    } else {
        Ok(fs::remove_file(resolved)?)
    }
}

/// Rename (move) a file or directory within the scope.
/// Both `from` and `to` must be within the base scope.
/// Uses `std::fs::rename` which is atomic on same-filesystem.
pub fn rename(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    let resolved_from = safe_resolve(base, from)?;
    let resolved_to = safe_resolve_for_create(base, to)?;

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
/// Both `from` and `to` must be within the base scope.
/// Only files can be copied; use mkdir + recursive copy for directories.
pub fn copy(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    let resolved_from = safe_resolve(base, from)?;
    let resolved_to = safe_resolve_for_create(base, to)?;

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

fn ensure_scoped_directory(base: &Path, directory: &Path) -> Result<(), FsError> {
    let canonical_base = canonical_base_dir(base)?;
    let relative = directory
        .strip_prefix(&canonical_base)
        .map_err(|_| FsError::OutOfScope)?;
    let mut current = canonical_base.clone();

    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(FsError::Security(format!(
                        "symlink component is not allowed: '{}'",
                        current.display()
                    )));
                }
                if !metadata.is_dir() {
                    return Err(FsError::Security(format!(
                        "path component is not a directory: '{}'",
                        current.display()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
            }
            Err(error) => return Err(FsError::Io(error)),
        }

        let canonical_current = current.canonicalize()?;
        if !canonical_current.starts_with(&canonical_base) {
            return Err(FsError::OutOfScope);
        }
        current = canonical_current;
    }

    Ok(())
}

fn ensure_scoped_parent_dirs(base: &Path, resolved: &Path) -> Result<(), FsError> {
    let Some(parent) = resolved.parent() else {
        return Err(FsError::Security(
            "Cannot resolve parent directory".to_string(),
        ));
    };
    ensure_scoped_directory(base, parent)
}

fn ensure_not_symlink(path: &Path) -> Result<(), FsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(FsError::Security(format!(
            "symlink targets are not allowed: '{}'",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(FsError::Io(error)),
    }
}

fn canonical_base_dir(base: &Path) -> Result<PathBuf, FsError> {
    base.canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))
}

#[cfg(test)]
mod tests;
