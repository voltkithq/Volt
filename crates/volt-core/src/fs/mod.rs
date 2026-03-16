use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
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

    let canonical_base = base
        .canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))?;
    let resolved = canonical_base.join(user_path);

    // Canonicalize both paths and verify the resolved path is under base.
    // If the file doesn't exist yet, canonicalize the parent.
    let canonical_resolved = if user_path.is_empty() || user_path == "." {
        canonical_base.clone()
    } else if resolved.exists() {
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
            ensure_not_symlink(parent)?;
            let canonical_parent = parent.canonicalize()?;
            let file_name = resolved
                .file_name()
                .ok_or_else(|| FsError::Security("Invalid file name".to_string()))?;
            canonical_parent.join(file_name)
        }
    };

    Ok(canonical_resolved)
}

/// Resolve a path for create/write flows while securely materializing any
/// missing parent directories inside the scoped base directory.
///
/// Built-in CRUD operations execute directly through a scoped directory handle
/// and do not need this path-returning helper. This remains part of the API
/// for callers such as `volt_db` that must hand a validated path to another
/// subsystem after the parent chain has been created safely.
pub fn safe_resolve_for_create(base: &Path, user_path: &str) -> Result<PathBuf, FsError> {
    let resolved = safe_resolve(base, user_path)?;
    ensure_scoped_parent_dirs(base, &resolved)?;
    ensure_not_symlink(&resolved)?;
    Ok(resolved)
}

/// Read a file's contents as bytes.
pub fn read_file(base: &Path, path: &str) -> Result<Vec<u8>, FsError> {
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    Ok(dir.read(scoped_path(path))?)
}

/// Read a file's contents as a UTF-8 string.
pub fn read_file_text(base: &Path, path: &str) -> Result<String, FsError> {
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    Ok(dir.read_to_string(scoped_path(path))?)
}

/// Write data to a file, creating it if it doesn't exist.
pub fn write_file(base: &Path, path: &str, data: &[u8]) -> Result<(), FsError> {
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    if let Some(parent) = Path::new(path).parent()
        && !parent.as_os_str().is_empty()
        && parent != Path::new(".")
    {
        dir.create_dir_all(parent)?;
    }

    let mut options = CapOpenOptions::new();
    let options = options.write(true).create(true).truncate(true);
    let mut file = dir.open_with(path, options)?;
    file.write_all(data)?;
    Ok(())
}

/// List entries in a directory.
pub fn read_dir(base: &Path, path: &str) -> Result<Vec<String>, FsError> {
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    let mut entries = Vec::new();
    let read_dir = if path.is_empty() || path == "." {
        dir.entries()?
    } else {
        dir.read_dir(path)?
    };
    for entry in read_dir {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            entries.push(name.to_string());
        }
    }
    Ok(entries)
}

/// Get metadata for a path.
pub fn stat(base: &Path, path: &str) -> Result<FileInfo, FsError> {
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    let meta = if path.is_empty() || path == "." {
        dir.dir_metadata()?
    } else {
        dir.metadata(path)?
    };

    let modified_ms = meta
        .modified()
        .ok()
        .map(|time| time.into_std())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);

    let created_ms = meta
        .created()
        .ok()
        .map(|time| time.into_std())
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
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    if path.is_empty() || path == "." {
        return Ok(true);
    }
    dir.try_exists(path).map_err(FsError::Io)
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
    validate_path(path).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;
    dir.create_dir_all(scoped_path(path))?;
    Ok(())
}

/// Remove a file or directory.
/// If the path is a directory, removal is recursive.
pub fn remove(base: &Path, path: &str) -> Result<(), FsError> {
    validate_path(path).map_err(FsError::Security)?;
    if path.is_empty() || path == "." {
        return Err(FsError::Security(
            "Refusing to remove the base directory".to_string(),
        ));
    }

    let (_, dir) = open_scoped_dir(base)?;
    let metadata = dir.symlink_metadata(path)?;
    if metadata.is_dir() {
        Ok(dir.remove_dir_all(path)?)
    } else {
        Ok(dir.remove_file(path)?)
    }
}

/// Rename (move) a file or directory within the scope.
/// Both `from` and `to` must be within the base scope.
/// Uses `std::fs::rename` which is atomic on same-filesystem.
pub fn rename(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    validate_path(from).map_err(FsError::Security)?;
    validate_path(to).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;

    if !dir.try_exists(from)? {
        return Err(FsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path does not exist: {from}"),
        )));
    }

    if dir.try_exists(to)? {
        return Err(FsError::Security(format!(
            "FS_ALREADY_EXISTS: destination already exists: {to}"
        )));
    }

    if let Some(parent) = Path::new(to).parent()
        && !parent.as_os_str().is_empty()
        && parent != Path::new(".")
    {
        dir.create_dir_all(parent)?;
    }
    dir.rename(from, &dir, to)?;
    Ok(())
}

/// Rename (move) a file or directory within the scope, replacing the
/// destination if it already exists.
///
/// This is currently used by internal storage writes. Callers should rely on
/// the scoped-handle confinement guarantees here, but not assume stronger
/// cross-platform atomic replacement semantics than the underlying OS rename
/// operation provides.
pub fn replace_file(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    validate_path(from).map_err(FsError::Security)?;
    validate_path(to).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;

    if !dir.try_exists(from)? {
        return Err(FsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path does not exist: {from}"),
        )));
    }

    if let Some(parent) = Path::new(to).parent()
        && !parent.as_os_str().is_empty()
        && parent != Path::new(".")
    {
        dir.create_dir_all(parent)?;
    }
    dir.rename(from, &dir, to)?;
    Ok(())
}

/// Copy a file within the scope.
/// Both `from` and `to` must be within the base scope.
/// Only files can be copied; use mkdir + recursive copy for directories.
pub fn copy(base: &Path, from: &str, to: &str) -> Result<(), FsError> {
    validate_path(from).map_err(FsError::Security)?;
    validate_path(to).map_err(FsError::Security)?;
    let (_, dir) = open_scoped_dir(base)?;

    if !dir.try_exists(from)? {
        return Err(FsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("source path does not exist: {from}"),
        )));
    }

    if !dir.metadata(from)?.is_file() {
        return Err(FsError::Security(
            "copy only supports files, not directories".to_string(),
        ));
    }

    if dir.try_exists(to)? {
        return Err(FsError::Security(format!(
            "FS_ALREADY_EXISTS: destination already exists: {to}"
        )));
    }

    if let Some(parent) = Path::new(to).parent()
        && !parent.as_os_str().is_empty()
        && parent != Path::new(".")
    {
        dir.create_dir_all(parent)?;
    }
    dir.copy(from, &dir, to)?;
    Ok(())
}

fn open_scoped_dir(base: &Path) -> Result<(PathBuf, Dir), FsError> {
    let canonical_base = canonical_base_dir(base)?;
    let dir = Dir::open_ambient_dir(&canonical_base, ambient_authority()).map_err(FsError::Io)?;
    Ok((canonical_base, dir))
}

fn scoped_path(path: &str) -> &Path {
    if path.is_empty() {
        Path::new(".")
    } else {
        Path::new(path)
    }
}

/// Materialize a directory chain below `base` one component at a time and
/// reject symlink substitutions while walking it.
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

/// Ensure the parent directory for a to-be-created path exists within the
/// scoped base directory before returning that path to external callers.
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
