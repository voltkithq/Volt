use std::path::PathBuf;

use serde_json::{Value, json};

use super::host_api_fs::perform_fs_request;
use super::{PLUGIN_ACCESS_ERROR_CODE, PLUGIN_FS_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::{lock_error, required_string, unavailable_plugin};

impl PluginManager {
    pub(super) fn handle_request_access(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let options = AccessRequestOptions::parse(payload)?;
        let dialog_request = {
            let registry = self.inner.registry.lock().map_err(lock_error)?;
            let Some(record) = registry.plugins.get(plugin_id) else {
                return Err(unavailable_plugin(plugin_id));
            };
            super::AccessDialogRequest {
                title: format_dialog_title(&record.manifest.name, &options),
                directory: options.directory,
                multiple: options.multiple,
            }
        };

        let Some(path) = self
            .inner
            .access_picker
            .pick_path(dialog_request)
            .map_err(access_error)?
        else {
            return Ok(Value::Null);
        };

        let grant_id = volt_core::grant_store::create_grant(path.clone()).map_err(|error| {
            PluginRuntimeError {
                code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
                message: error.to_string(),
            }
        })?;
        volt_core::plugin_grant_registry::delegate_grant(plugin_id, &grant_id).map_err(
            |error| PluginRuntimeError {
                code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
                message: error.to_string(),
            },
        )?;

        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.delegated_grants.insert(grant_id.clone());

        Ok(json!({
            "grantId": grant_id,
            "path": path.display().to_string(),
        }))
    }

    pub(super) fn handle_grant_fs_request(
        &self,
        plugin_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let grant_id = required_string(payload, "grantId")?;
        let path = required_string(payload, "path")?;
        let base_dir = self.resolve_delegated_grant_root(plugin_id, &grant_id)?;
        perform_fs_request(&base_dir, operation, &path, payload)
    }

    fn resolve_delegated_grant_root(
        &self,
        plugin_id: &str,
        grant_id: &str,
    ) -> Result<PathBuf, PluginRuntimeError> {
        let registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        if !record.delegated_grants.contains(grant_id)
            || !volt_core::plugin_grant_registry::is_delegated(plugin_id, grant_id)
        {
            return Err(PluginRuntimeError {
                code: PLUGIN_FS_ERROR_CODE.to_string(),
                message: format!("grant '{grant_id}' is not delegated to plugin '{plugin_id}'"),
            });
        }
        volt_core::grant_store::resolve_grant(grant_id).map_err(|error| PluginRuntimeError {
            code: PLUGIN_FS_ERROR_CODE.to_string(),
            message: error.to_string(),
        })
    }
}

#[derive(Debug)]
struct AccessRequestOptions {
    title: Option<String>,
    directory: bool,
    multiple: bool,
}

impl AccessRequestOptions {
    fn parse(payload: &Value) -> Result<Self, PluginRuntimeError> {
        let object = payload.as_object().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
            message: "payload must be an object".to_string(),
        })?;
        let title = object
            .get("title")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let directory = object
            .get("directory")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let multiple = object
            .get("multiple")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Ok(Self {
            title,
            directory,
            multiple,
        })
    }
}

fn format_dialog_title(plugin_name: &str, options: &AccessRequestOptions) -> String {
    let resource = if options.directory { "folder" } else { "file" };
    match &options.title {
        Some(title) => format!("Plugin '{plugin_name}' wants to access a {resource}: {title}"),
        None => format!("Plugin '{plugin_name}' wants to access a {resource}"),
    }
}

fn access_error(message: String) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
        message,
    }
}
