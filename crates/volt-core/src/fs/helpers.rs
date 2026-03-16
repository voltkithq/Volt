use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::fs;
use std::path::{Path, PathBuf};

use super::FsError;

pub(super) fn open_scoped_dir(base: &Path) -> Result<(PathBuf, Dir), FsError> {
    let canonical_base = canonical_base_dir(base)?;
    let dir = Dir::open_ambient_dir(&canonical_base, ambient_authority()).map_err(FsError::Io)?;
    Ok((canonical_base, dir))
}

pub(super) fn scoped_path(path: &str) -> &Path {
    if path.is_empty() {
        Path::new(".")
    } else {
        Path::new(path)
    }
}

/// Materialize a directory chain below `base` one component at a time and
/// reject symlink substitutions while walking it.
pub(super) fn ensure_scoped_directory(base: &Path, directory: &Path) -> Result<(), FsError> {
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
pub(super) fn ensure_scoped_parent_dirs(base: &Path, resolved: &Path) -> Result<(), FsError> {
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

pub(super) fn canonical_base_dir(base: &Path) -> Result<PathBuf, FsError> {
    base.canonicalize()
        .map_err(|_| FsError::Security("Base directory does not exist".to_string()))
}
