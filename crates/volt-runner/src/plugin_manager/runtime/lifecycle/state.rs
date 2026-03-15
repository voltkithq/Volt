use std::time::Duration;

use serde_json::{Value, json};

use crate::plugin_manager::{
    PLUGIN_AUTO_DISABLED_CODE, PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE,
    PluginLifecycleEvent, PluginManager, PluginRecord, PluginRegistry, PluginRuntimeError,
    PluginState, now_ms,
};

const MAX_CONSECUTIVE_FAILURES: u32 = 3;

impl PluginManager {
    pub(super) fn transition_plugin(
        &self,
        plugin_id: &str,
        next_state: PluginState,
    ) -> Result<(), PluginRuntimeError> {
        let event = {
            let mut registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin registry is unavailable".to_string(),
            })?;
            self.transition_plugin_locked(&mut registry, plugin_id, next_state)?
        };
        self.emit_lifecycle_event(event);
        Ok(())
    }

    pub(in crate::plugin_manager) fn transition_plugin_locked(
        &self,
        registry: &mut PluginRegistry,
        plugin_id: &str,
        next_state: PluginState,
    ) -> Result<PluginLifecycleEvent, PluginRuntimeError> {
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not registered"),
            })?;
        transition_record(plugin_id, record, next_state)
    }

    pub(crate) fn fail_plugin(
        &self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) {
        let events = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            self.fail_plugin_locked(&mut registry, plugin_id, code, message, details, stderr)
                .unwrap_or_default()
        };
        for event in events {
            self.emit_lifecycle_event(event);
        }
    }

    pub(in crate::plugin_manager) fn fail_plugin_locked(
        &self,
        registry: &mut PluginRegistry,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) -> Result<Vec<PluginLifecycleEvent>, PluginRuntimeError> {
        crate::plugin_manager::host_api_helpers::clear_plugin_registrations_locked(
            registry, plugin_id,
        );
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not registered"),
            })?;
        record.process = None;
        record.pending_requests = 0;

        let (transition, error, failures) = record
            .lifecycle
            .fail(
                plugin_id,
                code,
                message,
                details,
                stderr,
                self.inner.error_history_limit,
            )
            .map_err(runtime_error)?;
        let mut events = vec![PluginLifecycleEvent {
            plugin_id: plugin_id.to_string(),
            previous_state: transition.previous_state,
            new_state: transition.new_state,
            timestamp: transition.timestamp_ms,
            error: Some(error),
        }];

        if failures >= MAX_CONSECUTIVE_FAILURES {
            events.push(auto_disable_record(
                plugin_id,
                record,
                self.inner.error_history_limit,
                failures,
            )?);
        }

        Ok(events)
    }

    pub(in crate::plugin_manager) fn emit_lifecycle_event(&self, event: PluginLifecycleEvent) {
        self.inner.lifecycle_bus.emit(event);
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

    pub(super) fn activation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.activation_timeout_ms)
    }

    pub(super) fn deactivation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.deactivation_timeout_ms)
    }
}

fn transition_record(
    plugin_id: &str,
    record: &mut PluginRecord,
    next_state: PluginState,
) -> Result<PluginLifecycleEvent, PluginRuntimeError> {
    let transition = record
        .lifecycle
        .transition(next_state)
        .map_err(runtime_error)?;
    Ok(PluginLifecycleEvent {
        plugin_id: plugin_id.to_string(),
        previous_state: transition.previous_state,
        new_state: transition.new_state,
        timestamp: transition.timestamp_ms,
        error: None,
    })
}

fn auto_disable_record(
    plugin_id: &str,
    record: &mut PluginRecord,
    max_errors: usize,
    failures: u32,
) -> Result<PluginLifecycleEvent, PluginRuntimeError> {
    let transition = record
        .lifecycle
        .transition(PluginState::Disabled)
        .map_err(runtime_error)?;
    let error = record.lifecycle.push_error(
        crate::plugin_manager::PluginError {
            plugin_id: plugin_id.to_string(),
            state: PluginState::Disabled,
            code: PLUGIN_AUTO_DISABLED_CODE.to_string(),
            message: format!("plugin auto-disabled after {failures} consecutive failures"),
            details: Some(json!({ "consecutiveFailures": failures })),
            stderr: None,
            timestamp_ms: transition.timestamp_ms,
        },
        max_errors,
    );
    Ok(PluginLifecycleEvent {
        plugin_id: plugin_id.to_string(),
        previous_state: transition.previous_state,
        new_state: transition.new_state,
        timestamp: transition.timestamp_ms,
        error: Some(error),
    })
}

fn runtime_error(message: String) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
        message,
    }
}
