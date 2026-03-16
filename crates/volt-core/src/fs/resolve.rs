use std::path::{Path, PathBuf};

use crate::security::validate_path;

use super::FsError;
use super::helpers::{ensure_not_symlink, ensure_scoped_parent_dirs};

/// Safely resolve a user-provided relative path against a base directory.
/// Rejects absolute paths, path traversal, and ensures the result is under base.
pub fn safe_resolve(base: &Path, user_path: &str) -> Result<PathBuf, FsError> {
    validate_path(user_path).map_err(FsError::Security)?;

    let canonical_base = base
        .canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))?;
    let resolved = canonical_base.join(user_path);

    let canonical_resolved = if user_path.is_empty() || user_path == "." {
        canonical_base.clone()
    } else if resolved.exists() {
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
            ensure_not_symlink(parent)?;
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
