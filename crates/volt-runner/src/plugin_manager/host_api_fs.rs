use std::path::PathBuf;

use serde_json::{Value, json};
use volt_core::fs;

use super::{PLUGIN_FS_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::{lock_error, required_string, unavailable_plugin};

impl PluginManager {
    pub(super) fn handle_fs_request(
        &self,
        plugin_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let path = required_string(payload, "path")?;
        let data_root = self.plugin_data_root(plugin_id)?;
        perform_fs_request(&data_root, operation, &path, payload)
    }

    pub(super) fn plugin_data_root(&self, plugin_id: &str) -> Result<PathBuf, PluginRuntimeError> {
        let registry = self.inner.registry.lock().map_err(lock_error)?;
        registry
            .plugins
            .get(plugin_id)
            .and_then(|record| record.data_root.clone())
            .ok_or_else(|| unavailable_plugin(plugin_id))
    }
}

pub(super) fn perform_fs_request(
    base_dir: &std::path::Path,
    operation: &str,
    path: &str,
    payload: &Value,
) -> Result<Value, PluginRuntimeError> {
    match operation {
        "read-file" => fs::read_file_text(base_dir, path)
            .map(Value::String)
            .map_err(fs_error),
        "write-file" => {
            let data = required_string(payload, "data")?;
            fs::write_file(base_dir, path, data.as_bytes())
                .map(|_| Value::Bool(true))
                .map_err(fs_error)
        }
        "read-dir" => fs::read_dir(base_dir, path)
            .map(|entries| json!(entries))
            .map_err(fs_error),
        "stat" => fs::stat(base_dir, path).map(stat_json).map_err(fs_error),
        "exists" => fs::exists(base_dir, path)
            .map(Value::Bool)
            .map_err(fs_error),
        "mkdir" => fs::mkdir(base_dir, path)
            .map(|_| Value::Bool(true))
            .map_err(fs_error),
        "remove" => fs::remove(base_dir, path)
            .map(|_| Value::Bool(true))
            .map_err(fs_error),
        _ => Err(PluginRuntimeError {
            code: PLUGIN_FS_ERROR_CODE.to_string(),
            message: format!("unsupported fs operation '{operation}'"),
        }),
    }
}

fn fs_error(error: fs::FsError) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_FS_ERROR_CODE.to_string(),
        message: error.to_string(),
    }
}

fn stat_json(info: fs::FileInfo) -> Value {
    json!({
        "size": info.size,
        "isFile": info.is_file,
        "isDir": info.is_dir,
        "readonly": info.readonly,
        "modifiedMs": info.modified_ms,
        "createdMs": info.created_ms,
    })
}
