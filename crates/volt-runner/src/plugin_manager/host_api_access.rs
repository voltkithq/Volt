use std::path::PathBuf;

use serde_json::{Value, json};

use super::host_api_fs::perform_fs_request;
use super::{PLUGIN_ACCESS_ERROR_CODE, PLUGIN_FS_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::{lock_error, required_string, unavailable_plugin};

impl PluginManager {
    pub(crate) fn delegate_grant(
        &self,
        plugin_id: &str,
        grant_id: &str,
    ) -> Result<(), PluginRuntimeError> {
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };

        volt_core::plugin_grant_registry::delegate_grant(plugin_id, grant_id)
            .map_err(access_registry_error)?;
        record.delegated_grants.insert(grant_id.to_string());
        Ok(())
    }

    pub(crate) fn revoke_grant(
        &self,
        plugin_id: &str,
        grant_id: &str,
    ) -> Result<(), PluginRuntimeError> {
        let process = {
            let mut registry = self.inner.registry.lock().map_err(lock_error)?;
            let Some(record) = registry.plugins.get_mut(plugin_id) else {
                return Err(unavailable_plugin(plugin_id));
            };
            record.delegated_grants.remove(grant_id);
            record.process.clone()
        };

        volt_core::plugin_grant_registry::revoke_grant(plugin_id, grant_id);

        if let Some(process) = process {
            process.send_event("plugin:grant-revoked", json!({ "grantId": grant_id }))?;
        }
        Ok(())
    }

    pub(crate) fn list_delegated_grants(
        &self,
        plugin_id: &str,
    ) -> Result<Vec<String>, PluginRuntimeError> {
        let registry = self.inner.registry.lock().map_err(lock_error)?;
        if !registry.plugins.contains_key(plugin_id) {
            return Err(unavailable_plugin(plugin_id));
        }
        Ok(volt_core::plugin_grant_registry::list_delegated_grants(
            plugin_id,
        ))
    }

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

        let grant_id =
            volt_core::grant_store::create_grant(path.clone()).map_err(access_registry_error)?;
        self.delegate_grant(plugin_id, &grant_id)?;

        Ok(json!({
            "grantId": grant_id,
            "path": path.display().to_string(),
        }))
    }

    pub(super) fn handle_bind_grant(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let grant_id = required_string(payload, "grantId")?;
        let path = self.resolve_delegated_grant_root(plugin_id, &grant_id)?;
        Ok(json!({
            "grantId": grant_id,
            "path": path.display().to_string(),
        }))
    }

    pub(super) fn handle_list_grants(&self, plugin_id: &str) -> Result<Value, PluginRuntimeError> {
        Ok(json!(self.list_delegated_grants(plugin_id)?))
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
        if !registry.plugins.contains_key(plugin_id) {
            return Err(unavailable_plugin(plugin_id));
        }
        drop(registry);

        if !volt_core::plugin_grant_registry::is_delegated(plugin_id, grant_id) {
            return Err(PluginRuntimeError {
                code: PLUGIN_FS_ERROR_CODE.to_string(),
                message: format!("grant '{grant_id}' is not delegated to plugin '{plugin_id}'"),
            });
        }
        volt_core::grant_store::resolve_grant(grant_id).map_err(fs_error)
    }
}

#[derive(Debug)]
struct AccessRequestOptions {
    title: Option<String>,
    directory: bool,
    multiple: bool,
}

const MAX_DIALOG_TITLE_LEN: usize = 100;

impl AccessRequestOptions {
    fn parse(payload: &Value) -> Result<Self, PluginRuntimeError> {
        let object = payload.as_object().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
            message: "payload must be an object".to_string(),
        })?;
        Ok(Self {
            title: object
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(sanitize_dialog_title),
            directory: object
                .get("directory")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            multiple: object
                .get("multiple")
                .and_then(Value::as_bool)
                .unwrap_or(false),
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

/// Strip Unicode control characters (Cc and Cf categories including RTL
/// overrides) and truncate to prevent dialog spoofing by malicious plugins.
fn sanitize_dialog_title(raw: &str) -> String {
    let clean: String = raw
        .chars()
        .filter(|ch| !ch.is_control() && !matches!(ch, '\u{200E}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'))
        .take(MAX_DIALOG_TITLE_LEN)
        .collect();
    clean
}

fn access_error(message: String) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
        message,
    }
}

fn access_registry_error(error: impl std::fmt::Display) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_ACCESS_ERROR_CODE.to_string(),
        message: error.to_string(),
    }
}

fn fs_error(error: impl std::fmt::Display) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_FS_ERROR_CODE.to_string(),
        message: error.to_string(),
    }
}
