use std::path::PathBuf;

use serde_json::{Value, json};
use volt_core::fs;

use super::{
    PLUGIN_FS_ERROR_CODE, PluginCommandRoute, PluginManager, PluginRoute, PluginRuntimeError,
};
use crate::plugin_manager::host_api_helpers::{
    lock_error, namespaced_command, namespaced_ipc, required_string, unavailable_plugin,
};

impl PluginManager {
    pub(super) fn register_plugin_command(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let command_id = required_string(payload, "id")?;
        let namespaced = namespaced_command(plugin_id, &command_id);
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        registry.commands.insert(
            namespaced,
            PluginCommandRoute {
                plugin_id: plugin_id.to_string(),
                command_id: command_id.clone(),
            },
        );
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| unavailable_plugin(plugin_id))?;
        record.registrations.commands.insert(command_id);
        Ok(Value::Bool(true))
    }

    pub(super) fn unregister_plugin_command(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let command_id = required_string(payload, "id")?;
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.registrations.commands.remove(&command_id);
        registry
            .commands
            .remove(&namespaced_command(plugin_id, &command_id));
        Ok(Value::Bool(true))
    }

    pub(super) fn subscribe_plugin_event(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let event_name = required_string(payload, "event")?;
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.registrations.event_subscriptions.insert(event_name);
        Ok(Value::Bool(true))
    }

    pub(super) fn unsubscribe_plugin_event(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let event_name = required_string(payload, "event")?;
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.registrations.event_subscriptions.remove(&event_name);
        Ok(Value::Bool(true))
    }

    pub(super) fn emit_plugin_event(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let event_name = required_string(payload, "event")?;
        let data = payload.get("data").cloned().unwrap_or(Value::Null);
        let subscribers = self.plugin_event_subscribers(&event_name, Some(plugin_id));
        self.dispatch_event_to_plugins(&subscribers, &event_name, data);
        Ok(Value::Bool(true))
    }

    pub(super) fn register_plugin_ipc(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let channel = required_string(payload, "channel")?;
        let namespaced = namespaced_ipc(plugin_id, &channel);
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        registry.ipc_handlers.insert(
            namespaced,
            PluginRoute {
                plugin_id: plugin_id.to_string(),
                method: channel.clone(),
            },
        );
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.registrations.ipc_handlers.insert(channel);
        Ok(Value::Bool(true))
    }

    pub(super) fn unregister_plugin_ipc(
        &self,
        plugin_id: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let channel = required_string(payload, "channel")?;
        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(unavailable_plugin(plugin_id));
        };
        record.registrations.ipc_handlers.remove(&channel);
        registry
            .ipc_handlers
            .remove(&namespaced_ipc(plugin_id, &channel));
        Ok(Value::Bool(true))
    }

    pub(super) fn handle_fs_request(
        &self,
        plugin_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let path = required_string(payload, "path")?;
        let data_root = self.plugin_data_root(plugin_id)?;
        match operation {
            "read-file" => fs::read_file_text(&data_root, &path)
                .map(Value::String)
                .map_err(fs_error),
            "write-file" => {
                let data = required_string(payload, "data")?;
                fs::write_file(&data_root, &path, data.as_bytes())
                    .map(|_| Value::Bool(true))
                    .map_err(fs_error)
            }
            "read-dir" => fs::read_dir(&data_root, &path)
                .map(|entries| json!(entries))
                .map_err(fs_error),
            "stat" => fs::stat(&data_root, &path).map(stat_json).map_err(fs_error),
            "exists" => fs::exists(&data_root, &path)
                .map(Value::Bool)
                .map_err(fs_error),
            "mkdir" => fs::mkdir(&data_root, &path)
                .map(|_| Value::Bool(true))
                .map_err(fs_error),
            "remove" => fs::remove(&data_root, &path)
                .map(|_| Value::Bool(true))
                .map_err(fs_error),
            _ => Err(PluginRuntimeError {
                code: PLUGIN_FS_ERROR_CODE.to_string(),
                message: format!("unsupported fs operation '{operation}'"),
            }),
        }
    }

    pub(super) fn handle_plugin_log(&self, plugin_id: &str, payload: Value) {
        let level = payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info");
        let message = payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("<missing plugin log message>");

        match level {
            "debug" => tracing::debug!(plugin_id = %plugin_id, "{message}"),
            "warn" => tracing::warn!(plugin_id = %plugin_id, "{message}"),
            "error" => tracing::error!(plugin_id = %plugin_id, "{message}"),
            _ => tracing::info!(plugin_id = %plugin_id, "{message}"),
        }
    }

    pub(super) fn plugin_data_root(&self, plugin_id: &str) -> Result<PathBuf, PluginRuntimeError> {
        let registry = self.inner.registry.lock().map_err(lock_error)?;
        registry
            .plugins
            .get(plugin_id)
            .and_then(|record| record.data_root.clone())
            .ok_or_else(|| unavailable_plugin(plugin_id))
    }

    pub(super) fn plugin_event_subscribers(
        &self,
        event_name: &str,
        source_plugin_id: Option<&str>,
    ) -> Vec<String> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        registry
            .plugins
            .iter()
            .filter(|(plugin_id, record)| {
                record
                    .registrations
                    .event_subscriptions
                    .contains(event_name)
                    && source_plugin_id
                        .map(|source| source == plugin_id.as_str())
                        .unwrap_or(true)
            })
            .map(|(plugin_id, _)| plugin_id.clone())
            .collect()
    }

    pub(super) fn dispatch_event_to_plugins(
        &self,
        plugin_ids: &[String],
        event_name: &str,
        data: Value,
    ) {
        for plugin_id in plugin_ids {
            let Ok(process) = self.ensure_plugin_running(plugin_id) else {
                continue;
            };
            let result =
                process.send_event("plugin:event", json!({ "event": event_name, "data": data }));
            if result.is_ok() {
                self.record_activity(plugin_id);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn registered_commands(&self) -> Vec<String> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut commands = registry.commands.keys().cloned().collect::<Vec<_>>();
        commands.sort();
        commands
    }

    #[cfg(test)]
    pub(super) fn registered_ipc_handlers(&self) -> Vec<String> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut channels = registry.ipc_handlers.keys().cloned().collect::<Vec<_>>();
        channels.sort();
        channels
    }

    #[cfg(test)]
    pub(super) fn has_event_subscription(&self, plugin_id: &str, event_name: &str) -> bool {
        self.inner.registry.lock().ok().is_some_and(|registry| {
            registry.plugins.get(plugin_id).is_some_and(|record| {
                record
                    .registrations
                    .event_subscriptions
                    .contains(event_name)
            })
        })
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
