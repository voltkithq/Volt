use crate::plugin_manager::{
    PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginManager, PluginRuntimeError,
    PluginState,
};

impl PluginManager {
    pub(crate) fn retry_plugin(&self, plugin_id: &str) -> Result<(), PluginRuntimeError> {
        let state = {
            let registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin registry is unavailable".to_string(),
            })?;
            registry
                .plugins
                .get(plugin_id)
                .map(|record| record.lifecycle.current_state())
                .ok_or_else(|| PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is not registered"),
                })?
        };
        if state != PluginState::Failed {
            return Err(PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not in failed state"),
            });
        }
        self.retry_failed_plugin(plugin_id).map(|_| ())
    }

    pub(crate) fn enable_plugin(&self, plugin_id: &str) -> Result<(), PluginRuntimeError> {
        let event = {
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
            if !record.enabled {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is disabled by configuration"),
                });
            }
            if record.lifecycle.current_state() != PluginState::Disabled {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is not disabled"),
                });
            }
            record.process = None;
            record.pending_requests = 0;
            record.lifecycle.reset_failures();
            self.transition_plugin_locked(&mut registry, plugin_id, PluginState::Validated)?
        };
        self.emit_lifecycle_event(event);
        Ok(())
    }
}
