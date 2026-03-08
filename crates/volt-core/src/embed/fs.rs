use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) const MAX_ASSET_RECURSION_DEPTH: usize = 64;

/// Recursively collect files from a directory into the assets map.
pub(super) fn collect_files(
    root: &Path,
    dir: &Path,
    assets: &mut HashMap<String, Vec<u8>>,
    visited_dirs: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), std::io::Error> {
    let canonical_root = std::fs::canonicalize(root)?;
    collect_files_inner(root, &canonical_root, dir, assets, visited_dirs, depth)
}

fn collect_files_inner(
    logical_root: &Path,
    canonical_root: &Path,
    dir: &Path,
    assets: &mut HashMap<String, Vec<u8>>,
    visited_dirs: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), std::io::Error> {
    if depth > MAX_ASSET_RECURSION_DEPTH {
        return Err(std::io::Error::other(format!(
            "asset directory recursion depth exceeds limit ({MAX_ASSET_RECURSION_DEPTH})"
        )));
    }

    let canonical_dir = std::fs::canonicalize(dir)?;
    ensure_within_root(canonical_root, &canonical_dir)?;
    if !visited_dirs.insert(canonical_dir.clone()) {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let canonical_path = std::fs::canonicalize(&path)?;
        ensure_within_root(canonical_root, &canonical_path)?;
        let metadata = std::fs::metadata(&canonical_path)?;

        if metadata.is_dir() {
            collect_files_inner(
                logical_root,
                canonical_root,
                &path,
                assets,
                visited_dirs,
                depth + 1,
            )?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(logical_root)
                .map_err(std::io::Error::other)?;
            // Normalize path separators to forward slashes.
            let key = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let data = std::fs::read(&canonical_path)?;
            assets.insert(key, data);
        }
    }

    Ok(())
}

fn ensure_within_root(canonical_root: &Path, path: &Path) -> Result<(), std::io::Error> {
    if path.starts_with(canonical_root) {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "asset path '{}' resolves outside asset root '{}'",
            path.display(),
            canonical_root.display()
        )))
    }
}
