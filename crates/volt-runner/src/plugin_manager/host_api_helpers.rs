use serde_json::Value;

use super::{PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginRuntimeError};
use crate::plugin_manager::process::{WireError, WireMessage, WireMessageType};

pub(super) fn namespaced_command(plugin_id: &str, command_id: &str) -> String {
    format!("plugin:{plugin_id}:{command_id}")
}

pub(super) fn namespaced_ipc(plugin_id: &str, channel: &str) -> String {
    format!("plugin:{plugin_id}:{channel}")
}

pub(super) fn namespaced_event(plugin_id: &str, event_name: &str) -> String {
    format!("plugin:{plugin_id}:{event_name}")
}

pub(super) fn plugin_event_subscription_key(event_name: &str) -> String {
    format!("plugin:*:{event_name}")
}

pub(super) fn host_event_subscription_key(event_name: &str) -> String {
    event_name.to_string()
}

pub(super) fn event_subscription_key(event_name: &str) -> String {
    if event_name.starts_with("plugin:") {
        event_name.to_string()
    } else if is_host_event(event_name) {
        host_event_subscription_key(event_name)
    } else {
        plugin_event_subscription_key(event_name)
    }
}

pub(super) fn is_host_event(event_name: &str) -> bool {
    ["app:", "menu:", "shortcut:", "tray:"]
        .iter()
        .any(|prefix| event_name.starts_with(prefix))
}

pub(super) fn success_response(id: &str, method: &str, payload: Value) -> WireMessage {
    WireMessage {
        message_type: WireMessageType::Response,
        id: id.to_string(),
        method: method.to_string(),
        payload: Some(payload),
        error: None,
    }
}

pub(super) fn error_response(id: &str, method: &str, code: &str, message: String) -> WireMessage {
    WireMessage {
        message_type: WireMessageType::Response,
        id: id.to_string(),
        method: method.to_string(),
        payload: None,
        error: Some(WireError {
            code: code.to_string(),
            message,
        }),
    }
}

pub(super) fn required_string(payload: &Value, key: &str) -> Result<String, PluginRuntimeError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("payload is missing required '{key}' string"),
        })
}

pub(super) fn lock_error(
    _: std::sync::PoisonError<std::sync::MutexGuard<'_, super::PluginRegistry>>,
) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
        message: "plugin registry is unavailable".to_string(),
    }
}

pub(super) fn unavailable_plugin(plugin_id: &str) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
        message: format!("plugin '{plugin_id}' is not available"),
    }
}

pub(super) fn clear_plugin_registrations_locked(
    registry: &mut super::PluginRegistry,
    plugin_id: &str,
) {
    let Some(record) = registry.plugins.get_mut(plugin_id) else {
        return;
    };

    for command_id in std::mem::take(&mut record.registrations.commands) {
        registry
            .commands
            .remove(&namespaced_command(plugin_id, &command_id));
    }
    record.registrations.event_subscriptions.clear();
    for channel in std::mem::take(&mut record.registrations.ipc_handlers) {
        registry
            .ipc_handlers
            .remove(&namespaced_ipc(plugin_id, &channel));
    }
    if !record.delegated_grants.is_empty() {
        volt_core::plugin_grant_registry::revoke_all_grants(plugin_id);
        record.delegated_grants.clear();
    }
    record.storage_reconciled = false;
}
