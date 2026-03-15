use std::time::Duration;

use serde_json::Value;
use serde_json::json;

use super::PLUGIN_COMMAND_NOT_FOUND_CODE;
use super::{PLUGIN_RUNTIME_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::{
    error_response, host_event_subscription_key, lock_error, success_response,
};
use crate::plugin_manager::process::{WireMessage, WireMessageType};

impl PluginManager {
    pub(super) fn handle_plugin_message(
        &self,
        plugin_id: &str,
        message: WireMessage,
    ) -> Option<WireMessage> {
        match message.message_type {
            WireMessageType::Request => Some(self.handle_plugin_request(
                plugin_id,
                &message.id,
                &message.method,
                message.payload.unwrap_or(Value::Null),
            )),
            WireMessageType::Event if message.method == "plugin:log" => {
                self.handle_plugin_log(plugin_id, message.payload.unwrap_or(Value::Null));
                None
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_command(
        &self,
        namespaced_command: &str,
        args: Value,
        timeout: Duration,
    ) -> Result<Value, PluginRuntimeError> {
        let route = {
            let registry = self.inner.registry.lock().map_err(lock_error)?;
            registry
                .commands
                .get(namespaced_command)
                .cloned()
                .ok_or_else(|| PluginRuntimeError {
                    code: PLUGIN_COMMAND_NOT_FOUND_CODE.to_string(),
                    message: format!("plugin command '{namespaced_command}' is not registered"),
                })?
        };

        let process = self.ensure_plugin_running(&route.plugin_id)?;
        self.mark_request_started(&route.plugin_id);
        let response = process.request(
            "plugin:invoke-command",
            json!({ "id": route.command_id, "args": args }),
            timeout,
        );
        self.mark_request_finished(&route.plugin_id);

        match response {
            Ok(message) if message.error.is_none() => {
                self.record_activity(&route.plugin_id);
                Ok(message.payload.unwrap_or(Value::Null))
            }
            Ok(message) => {
                let error = message.error.expect("checked above");
                Err(PluginRuntimeError {
                    code: error.code,
                    message: error.message,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn dispatch_host_event(&self, event_name: &str, data: Value) {
        let subscribers =
            self.plugin_event_subscribers(&host_event_subscription_key(event_name), None);
        self.dispatch_event_to_plugins(&subscribers, event_name, data);
    }

    fn handle_plugin_request(
        &self,
        plugin_id: &str,
        id: &str,
        method: &str,
        payload: Value,
    ) -> WireMessage {
        let response = match method {
            "plugin:register-command" => self.register_plugin_command(plugin_id, &payload),
            "plugin:unregister-command" => self.unregister_plugin_command(plugin_id, &payload),
            "plugin:subscribe-event" => self.subscribe_plugin_event(plugin_id, &payload),
            "plugin:unsubscribe-event" => self.unsubscribe_plugin_event(plugin_id, &payload),
            "plugin:emit-event" => self.emit_plugin_event(plugin_id, &payload),
            "plugin:register-ipc" => self.register_plugin_ipc(plugin_id, &payload),
            "plugin:unregister-ipc" => self.unregister_plugin_ipc(plugin_id, &payload),
            "plugin:fs:read-file" => self.handle_fs_request(plugin_id, "read-file", &payload),
            "plugin:fs:write-file" => self.handle_fs_request(plugin_id, "write-file", &payload),
            "plugin:fs:read-dir" => self.handle_fs_request(plugin_id, "read-dir", &payload),
            "plugin:fs:stat" => self.handle_fs_request(plugin_id, "stat", &payload),
            "plugin:fs:exists" => self.handle_fs_request(plugin_id, "exists", &payload),
            "plugin:fs:mkdir" => self.handle_fs_request(plugin_id, "mkdir", &payload),
            "plugin:fs:remove" => self.handle_fs_request(plugin_id, "remove", &payload),
            _ => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: format!("plugin host method '{method}' is not supported"),
            }),
        };

        match response {
            Ok(payload) => success_response(id, method, payload),
            Err(error) => error_response(id, method, &error.code, error.message),
        }
    }
}
