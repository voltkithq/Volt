use std::time::Duration;

use serde_json::Value;

use crate::plugin_manager::{
    PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginManager, PluginRuntimeError,
    PluginState, now_ms,
};

impl PluginManager {
    pub(super) fn transition_plugin(
        &self,
        plugin_id: &str,
        next_state: PluginState,
    ) -> Result<(), PluginRuntimeError> {
        let mut registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin registry is unavailable".to_string(),
        })?;
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not registered"),
            })?;
        record
            .lifecycle
            .transition(next_state)
            .map_err(|message| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message,
            })
    }

    pub(in crate::plugin_manager) fn record_activity(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    pub(in crate::plugin_manager) fn mark_request_started(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.pending_requests += 1;
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    pub(in crate::plugin_manager) fn mark_request_finished(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.pending_requests = record.pending_requests.saturating_sub(1);
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    pub(in crate::plugin_manager) fn fail_plugin(
        &self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            record
                .lifecycle
                .fail(plugin_id, code, message, details, stderr);
        }
    }

    pub(super) fn activation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.activation_timeout_ms)
    }

    pub(super) fn deactivation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.deactivation_timeout_ms)
    }
}
