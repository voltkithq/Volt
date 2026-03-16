use std::fs;
use std::path::{Path, PathBuf};

use super::FsError;
use crate::security::validate_path;

/// Verify that an opened file descriptor actually resides under the expected
/// base directory. This closes the TOCTOU gap between path validation and
/// the actual file operation: even if the path was swapped (e.g., via a
/// symlink race) between `safe_resolve` and `open`, the post-open check
/// catches the escape.
pub(super) fn verify_opened_path(opened: &Path, canonical_base: &Path) -> Result<(), FsError> {
    let actual = opened
        .canonicalize()
        .map_err(|_| FsError::Security("cannot verify opened path".to_string()))?;
    if !actual.starts_with(canonical_base) {
        return Err(FsError::OutOfScope);
    }
    Ok(())
}

/// Canonicalize the base once and return it for reuse in post-open checks.
pub(super) fn canonical_base(base: &Path) -> Result<PathBuf, FsError> {
    base.canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))
}

/// Safely resolve a user-provided relative path against a base directory.
/// Rejects absolute paths, path traversal, and ensures the result is under base.
pub fn safe_resolve(base: &Path, user_path: &str) -> Result<PathBuf, FsError> {
    validate_path(user_path).map_err(FsError::Security)?;

    let resolved = base.join(user_path);

    let canonical_base = base
        .canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))?;

    let canonical_resolved = if resolved.exists() {
        resolved.canonicalize()?
    } else {
        let parent = resolved
            .parent()
            .ok_or_else(|| FsError::Security("Cannot resolve parent directory".to_string()))?;

        if !parent.exists() {
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

pub(super) fn ensure_scoped_directory(base: &Path, directory: &Path) -> Result<(), FsError> {
    let canonical_base = canonical_base(base)?;
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

pub(super) fn ensure_not_symlink(path: &Path) -> Result<(), FsError> {
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
