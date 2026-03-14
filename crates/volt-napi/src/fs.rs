use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use std::path::Path;
use volt_core::fs;
use volt_core::grant_store;
use volt_core::permissions::Permission;
use volt_core::watcher;

use crate::permissions::require_permission;

/// File metadata returned by fs_stat.
#[napi(object)]
pub struct VoltFileInfo {
    /// File size in bytes.
    pub size: i64,
    /// Whether the path is a file.
    pub is_file: bool,
    /// Whether the path is a directory.
    pub is_dir: bool,
    /// Whether the file is read-only.
    pub readonly: bool,
    /// Last modification time as milliseconds since Unix epoch.
    pub modified_ms: f64,
    /// Creation time as milliseconds since Unix epoch, or null if unavailable.
    pub created_ms: Option<f64>,
}

/// Read a file as raw bytes. Path is relative to the base scope directory.
#[napi]
pub fn fs_read_file(base_dir: String, path: String) -> napi::Result<Buffer> {
    require_permission(Permission::FileSystem)?;
    let data = fs::read_file(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs read failed: {e}")))?;
    Ok(data.into())
}

/// Read a file as a UTF-8 string. Path is relative to the base scope directory.
#[napi]
pub fn fs_read_file_text(base_dir: String, path: String) -> napi::Result<String> {
    require_permission(Permission::FileSystem)?;
    fs::read_file_text(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs read text failed: {e}")))
}

/// Write data to a file. Path is relative to the base scope directory.
#[napi]
pub fn fs_write_file(base_dir: String, path: String, data: Buffer) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    fs::write_file(Path::new(&base_dir), &path, &data)
        .map_err(|e| napi::Error::from_reason(format!("fs write failed: {e}")))
}

/// List entries in a directory. Path is relative to the base scope directory.
#[napi]
pub fn fs_read_dir(base_dir: String, path: String) -> napi::Result<Vec<String>> {
    require_permission(Permission::FileSystem)?;
    fs::read_dir(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs read dir failed: {e}")))
}

/// Get file/directory metadata. Path is relative to the base scope directory.
#[napi]
pub fn fs_stat(base_dir: String, path: String) -> napi::Result<VoltFileInfo> {
    require_permission(Permission::FileSystem)?;
    let info = fs::stat(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs stat failed: {e}")))?;
    let size = i64::try_from(info.size).map_err(|_| {
        napi::Error::from_reason(format!(
            "fs stat failed: file size {} exceeds i64::MAX",
            info.size
        ))
    })?;

    Ok(VoltFileInfo {
        size,
        is_file: info.is_file,
        is_dir: info.is_dir,
        readonly: info.readonly,
        modified_ms: info.modified_ms,
        created_ms: info.created_ms,
    })
}

/// Check whether a path exists within the base scope directory.
#[napi]
pub fn fs_exists(base_dir: String, path: String) -> napi::Result<bool> {
    require_permission(Permission::FileSystem)?;
    fs::exists(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs exists failed: {e}")))
}

/// Resolve a grant ID to its root path string.
/// Returns the absolute path for the grant, or throws if the grant is invalid.
#[napi]
pub fn fs_resolve_grant(grant_id: String) -> napi::Result<String> {
    require_permission(Permission::FileSystem)?;
    let path = grant_store::resolve_grant(&grant_id)
        .map_err(|e| napi::Error::from_reason(format!("{e}")))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Create a directory (and parents). Path is relative to the base scope directory.
#[napi]
pub fn fs_mkdir(base_dir: String, path: String) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    fs::mkdir(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs mkdir failed: {e}")))
}

/// Remove a file or directory. Path is relative to the base scope directory.
#[napi]
pub fn fs_remove(base_dir: String, path: String) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    fs::remove(Path::new(&base_dir), &path)
        .map_err(|e| napi::Error::from_reason(format!("fs remove failed: {e}")))
}

/// Rename (move) a file or directory within the scope.
#[napi]
pub fn fs_rename(base_dir: String, from: String, to: String) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    fs::rename(Path::new(&base_dir), &from, &to)
        .map_err(|e| napi::Error::from_reason(format!("fs rename failed: {e}")))
}

/// Copy a file within the scope.
#[napi]
pub fn fs_copy(base_dir: String, from: String, to: String) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    fs::copy(Path::new(&base_dir), &from, &to)
        .map_err(|e| napi::Error::from_reason(format!("fs copy failed: {e}")))
}

/// Start watching a directory for changes. Returns a watcher ID.
#[napi]
pub fn fs_watch_start(
    base_dir: String,
    subpath: String,
    recursive: bool,
    debounce_ms: f64,
) -> napi::Result<String> {
    require_permission(Permission::FileSystem)?;
    let base = Path::new(&base_dir);
    let target = if subpath.is_empty() {
        base.to_path_buf()
    } else {
        fs::safe_resolve(base, &subpath)
            .map_err(|e| napi::Error::from_reason(format!("fs watch scope error: {e}")))?
    };
    watcher::start_watch(target, recursive, debounce_ms as u64)
        .map_err(|e| napi::Error::from_reason(format!("fs watch failed: {e}")))
}

/// Drain all pending events from a watcher. Returns a JSON-serializable array.
#[napi]
pub fn fs_watch_poll(watcher_id: String) -> napi::Result<Vec<serde_json::Value>> {
    require_permission(Permission::FileSystem)?;
    let events = watcher::drain_events(&watcher_id)
        .map_err(|e| napi::Error::from_reason(format!("fs watch poll failed: {e}")))?;
    events
        .into_iter()
        .map(|e| serde_json::to_value(e).map_err(|e| napi::Error::from_reason(format!("{e}"))))
        .collect()
}

/// Stop a watcher and release resources.
#[napi]
pub fn fs_watch_close(watcher_id: String) -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    watcher::stop_watch(&watcher_id)
        .map_err(|e| napi::Error::from_reason(format!("fs watch close failed: {e}")))
}
