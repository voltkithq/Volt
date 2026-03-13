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
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;

    #[cfg(unix)]
    fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(src, dst)
    }

    #[cfg(windows)]
    fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(src, dst)
    }

    #[test]
    fn test_path_traversal_rejected() {
        let base = env::temp_dir();
        assert!(safe_resolve(&base, "../../etc/passwd").is_err());
        assert!(safe_resolve(&base, "/etc/passwd").is_err());
    }

    #[test]
    fn test_valid_relative_path() {
        let base = env::temp_dir();
        // This should succeed since temp dir exists
        let result = safe_resolve(&base, "test_volt_file.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
    }

    #[test]
    fn test_write_and_read() {
        let base = env::temp_dir();
        let test_dir = "volt_test_fs";
        let test_file = &format!("{test_dir}/test.txt");

        // Write
        write_file(&base, test_file, b"hello volt").unwrap();

        // Read
        let content = read_file_text(&base, test_file).unwrap();
        assert_eq!(content, "hello volt");

        // Stat
        let info = stat(&base, test_file).unwrap();
        assert!(info.is_file);
        assert_eq!(info.size, 10);

        // Read dir
        let entries = read_dir(&base, test_dir).unwrap();
        assert!(entries.contains(&"test.txt".to_string()));

        // Clean up
        remove(&base, test_dir).unwrap();
    }

    // ── Expanded tests ─────────────────────────────────────────────

    #[test]
    fn test_backslash_path_rejected() {
        let base = env::temp_dir();
        assert!(safe_resolve(&base, "\\etc\\passwd").is_err());
    }

    #[test]
    fn test_windows_drive_letter_rejected() {
        let base = env::temp_dir();
        assert!(safe_resolve(&base, "C:\\Windows\\System32").is_err());
        assert!(safe_resolve(&base, "D:\\data.txt").is_err());
    }

    #[test]
    fn test_stat_directory() {
        let base = env::temp_dir();
        let dir_name = "volt_test_stat_dir";
        mkdir(&base, dir_name).unwrap();

        let info = stat(&base, dir_name).unwrap();
        assert!(info.is_dir);
        assert!(!info.is_file);

        // Clean up
        remove(&base, dir_name).unwrap();
    }

    #[test]
    fn test_mkdir_nested() {
        let base = env::temp_dir();
        let nested = "volt_test_nested/a/b/c";
        mkdir(&base, nested).unwrap();

        let info = stat(&base, nested).unwrap();
        assert!(info.is_dir);

        // Clean up
        remove(&base, "volt_test_nested").unwrap();
    }

    #[test]
    fn test_remove_file() {
        let base = env::temp_dir();
        let file = "volt_test_remove_file.txt";
        write_file(&base, file, b"to be removed").unwrap();

        let resolved = safe_resolve(&base, file).unwrap();
        assert!(resolved.exists());

        remove(&base, file).unwrap();
        assert!(!resolved.exists());
    }

    #[test]
    fn test_read_dir_empty() {
        let base = env::temp_dir();
        let dir_name = "volt_test_empty_dir";
        mkdir(&base, dir_name).unwrap();

        let entries = read_dir(&base, dir_name).unwrap();
        assert!(entries.is_empty());

        // Clean up
        remove(&base, dir_name).unwrap();
    }

    #[test]
    fn test_read_nonexistent_file_error() {
        let base = env::temp_dir();
        let result = read_file(&base, "volt_definitely_does_not_exist_12345.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let base = env::temp_dir();
        let path = "volt_test_auto_parent/sub1/sub2/file.txt";
        write_file(&base, path, b"deep file").unwrap();

        let content = read_file_text(&base, path).unwrap();
        assert_eq!(content, "deep file");

        // Clean up
        remove(&base, "volt_test_auto_parent").unwrap();
    }

    #[test]
    fn test_safe_resolve_for_create_rejects_symlinked_parent_escape() {
        let base = env::temp_dir().join("volt_test_create_scope_guard");
        let outside = env::temp_dir().join("volt_test_create_scope_guard_outside");
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&outside).unwrap();

        if create_dir_symlink(&outside, &base.join("linked")).is_err() {
            let _ = fs::remove_dir_all(&base);
            let _ = fs::remove_dir_all(&outside);
            return;
        }

        let result = safe_resolve_for_create(&base, "linked/escape.txt");
        assert!(matches!(
            result,
            Err(FsError::Security(_)) | Err(FsError::OutOfScope)
        ));

        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn test_fs_error_display() {
        let e = FsError::Security("bad path".into());
        assert!(e.to_string().contains("bad path"));

        let e = FsError::OutOfScope;
        assert!(e.to_string().contains("outside"));
    }

    #[test]
    fn test_safe_resolve_allows_double_dot_inside_component() {
        let base = env::temp_dir();
        let result = safe_resolve(&base, "volt_test_a..b/file.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
    }

    #[test]
    fn test_remove_rejects_base_directory_targets() {
        let base = env::temp_dir().join("volt_test_remove_base_guard");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("keep.txt"), b"keep").unwrap();

        assert!(remove(&base, ".").is_err());
        assert!(remove(&base, "").is_err());
        assert!(base.exists());

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn test_stat_returns_timestamps() {
        let base = env::temp_dir();
        let file = "volt_test_stat_timestamps.txt";
        write_file(&base, file, b"timestamp test").unwrap();

        let info = stat(&base, file).unwrap();
        assert!(info.modified_ms > 0.0, "modified_ms should be positive");
        // created_ms may be None on some Linux filesystems, but should be
        // Some on Windows and macOS.
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        assert!(info.created_ms.is_some(), "created_ms should be available");

        // Clean up
        remove(&base, file).unwrap();
    }

    #[test]
    fn test_exists_returns_true_for_existing_file() {
        let base = env::temp_dir();
        let file = "volt_test_exists_true.txt";
        write_file(&base, file, b"exists").unwrap();

        assert!(exists(&base, file).unwrap());

        // Clean up
        remove(&base, file).unwrap();
    }

    #[test]
    fn test_exists_returns_false_for_missing_file() {
        let base = env::temp_dir();
        assert!(!exists(&base, "volt_test_exists_missing_12345.txt").unwrap());
    }

    #[test]
    fn test_exists_rejects_traversal() {
        let base = env::temp_dir();
        assert!(exists(&base, "../../etc/passwd").is_err());
    }
}
