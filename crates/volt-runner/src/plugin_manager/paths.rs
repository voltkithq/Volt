use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::PluginDiscoveryIssue;

const MANIFEST_FILE_NAME: &str = "volt-plugin.json";

pub(super) fn resolve_plugin_directory(path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(candidate)
    }
}

pub(super) fn collect_manifest_paths(
    directory: &Path,
    manifest_paths: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut visited = HashSet::new();
    collect_manifest_paths_inner(directory, manifest_paths, &mut visited)
}

fn collect_manifest_paths_inner(
    directory: &Path,
    manifest_paths: &mut Vec<PathBuf>,
    visited_directories: &mut HashSet<PathBuf>,
) -> std::io::Result<()> {
    let resolved = fs::canonicalize(directory)?;
    if !visited_directories.insert(resolved) {
        return Ok(());
    }

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() || (file_type.is_symlink() && path.is_dir()) {
            collect_manifest_paths_inner(&path, manifest_paths, visited_directories)?;
        } else if path.file_name().and_then(|value| value.to_str()) == Some(MANIFEST_FILE_NAME) {
            manifest_paths.push(path);
        }
    }

    Ok(())
}

pub(super) fn resolve_app_data_root(app_name: &str) -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| "failed to resolve app data directory".to_string())?;
    let root = base.join("volt").join(sanitize_app_namespace(app_name));
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create app data directory '{}': {error}",
            root.display()
        )
    })?;
    Ok(root)
}

pub(super) fn ensure_plugin_data_root(
    app_data_root: &Path,
    plugin_id: &str,
) -> Result<PathBuf, PluginDiscoveryIssue> {
    let data_root = app_data_root.join("plugins").join(plugin_id);
    fs::create_dir_all(&data_root).map_err(|error| PluginDiscoveryIssue {
        path: Some(data_root.clone()),
        message: format!("failed to create plugin data root: {error}"),
    })?;
    Ok(data_root)
}

fn sanitize_app_namespace(app_name: &str) -> String {
    let sanitized = app_name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let compact = sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if compact.is_empty() {
        "app".to_string()
    } else {
        compact
    }
}
