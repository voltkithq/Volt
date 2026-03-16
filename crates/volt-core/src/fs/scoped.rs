use cap_std::fs::OpenOptions as CapOpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::security::validate_path;

use super::helpers::{open_scoped_dir, scoped_path};
use super::{FileInfo, FsError};

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
