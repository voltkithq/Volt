use std::path::{Path, PathBuf};

use boa_engine::{Context, JsArgs, JsResult, JsValue};
use serde_json::json;
use volt_core::grant_store;
use volt_core::permissions::Permission;

use crate::modules::{format_js_error, fs_base_dir, require_permission};

pub(super) fn arg_string(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
) -> JsResult<String> {
    args.get_or_undefined(index)
        .to_string(context)
        .map(|s| s.to_std_string_escaped())
}

pub(super) fn require_fs_permission() -> Result<(), String> {
    require_permission(Permission::FileSystem).map_err(format_js_error)
}

pub(super) fn base_dir() -> Result<PathBuf, String> {
    require_fs_permission()?;
    fs_base_dir()
}

pub(super) fn scoped_base_dir(grant_id: &str) -> Result<PathBuf, String> {
    require_fs_permission()?;
    grant_store::resolve_grant(grant_id).map_err(|error| error.to_string())
}

pub(super) fn stat_json(info: volt_core::fs::FileInfo) -> serde_json::Value {
    json!({
        "size": info.size,
        "isFile": info.is_file,
        "isDir": info.is_dir,
        "readonly": info.readonly,
        "modifiedMs": info.modified_ms,
        "createdMs": info.created_ms,
    })
}

pub(super) fn with_base_dir<T>(op: impl FnOnce(&Path) -> Result<T, String>) -> Result<T, String> {
    let base = base_dir()?;
    op(&base)
}

pub(super) fn with_scoped_base_dir<T>(
    grant_id: &str,
    op: impl FnOnce(&Path) -> Result<T, String>,
) -> Result<T, String> {
    let base = scoped_base_dir(grant_id)?;
    op(&base)
}
