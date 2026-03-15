use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::plugin_manager::{
    PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginBootstrapConfig, PluginManager,
    PluginProcessHandle, PluginRuntimeError, PluginState, now_ms,
};

impl PluginManager {
    pub(super) fn ensure_plugin_running(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        let spawn_lock = {
            let registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin registry is unavailable".to_string(),
            })?;
            let record = registry
                .plugins
                .get(plugin_id)
                .ok_or_else(|| PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is not registered"),
                })?;
            record.spawn_lock.clone()
        };
        let _guard = spawn_lock.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("spawn lock for plugin '{plugin_id}' is poisoned"),
        })?;

        if let Some(process) = self.current_process(plugin_id) {
            self.record_activity(plugin_id);
            return Ok(process);
        }

        let bootstrap = self.prepare_spawn(plugin_id)?;
        let process = self.inner.factory.spawn(&bootstrap)?;
        let manager = self.clone();
        let plugin_id_for_exit = plugin_id.to_string();
        process.set_exit_listener(Arc::new(move |exit| {
            manager.handle_process_exit(&plugin_id_for_exit, exit);
        }));
        self.register_process(plugin_id, process.clone(), now_ms());

        if let Err(error) = process.wait_for_ready(self.activation_timeout()) {
            self.fail_plugin(
                plugin_id,
                &error.code,
                error.message,
                None,
                process.stderr_snapshot(),
            );
            let _ = process.kill();
            return Err(PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' failed before ready"),
            });
        }
        self.transition_plugin(plugin_id, PluginState::Loaded)?;

        if let Err(error) = process.activate(self.activation_timeout()) {
            self.fail_plugin(
                plugin_id,
                &error.code,
                error.message,
                None,
                process.stderr_snapshot(),
            );
            let _ = process.kill();
            return Err(PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' failed to activate"),
            });
        }
        self.transition_plugin(plugin_id, PluginState::Active)?;
        self.transition_plugin(plugin_id, PluginState::Running)?;
        self.record_activity(plugin_id);
        self.start_watchdog(plugin_id.to_string(), process.clone());

        Ok(process)
    }

    fn current_process(&self, plugin_id: &str) -> Option<Arc<dyn PluginProcessHandle>> {
        let Ok(registry) = self.inner.registry.lock() else {
            return None;
        };
        let record = registry.plugins.get(plugin_id)?;
        if matches!(
            record.lifecycle.current_state(),
            PluginState::Active | PluginState::Running
        ) {
            record.process.clone()
        } else {
            None
        }
    }

    fn prepare_spawn(&self, plugin_id: &str) -> Result<PluginBootstrapConfig, PluginRuntimeError> {
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

        match record.lifecycle.current_state() {
            PluginState::Validated | PluginState::Terminated => {
                record
                    .lifecycle
                    .transition(PluginState::Spawning)
                    .map_err(|message| PluginRuntimeError {
                        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                        message,
                    })?;
            }
            PluginState::Active | PluginState::Running => {}
            PluginState::Disabled => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is disabled"),
                });
            }
            PluginState::Failed => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is in failed state"),
                });
            }
            other => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: format!(
                        "plugin '{plugin_id}' cannot be spawned from state {:?}",
                        other
                    ),
                });
            }
        }

        let data_root = record.data_root.clone().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("plugin '{plugin_id}' is missing a data root"),
        })?;
        Ok(PluginBootstrapConfig {
            plugin_id: record.manifest.id.clone(),
            capabilities: record.effective_capabilities.iter().cloned().collect(),
            data_root: data_root.display().to_string(),
            delegated_grants: Vec::new(),
            host_ipc_settings: crate::plugin_manager::HostIpcSettings {
                heartbeat_interval_ms: self.inner.config.limits.heartbeat_interval_ms,
                heartbeat_timeout_ms: self.inner.config.limits.heartbeat_timeout_ms,
                call_timeout_ms: self.inner.config.limits.call_timeout_ms,
                max_inflight: 64,
                max_queue_depth: 256,
            },
        })
    }

    fn register_process(
        &self,
        plugin_id: &str,
        process: Arc<dyn PluginProcessHandle>,
        started_at_ms: u64,
    ) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.metrics.pid = process.process_id();
            record.metrics.started_at_ms = Some(started_at_ms);
            record.process = Some(process);
        }
    }

    fn transition_plugin(
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

    pub(in crate::plugin_manager) fn deactivate_plugin(&self, plugin_id: &str) {
        let process = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            let Some(record) = registry.plugins.get_mut(plugin_id) else {
                return;
            };
            if !matches!(
                record.lifecycle.current_state(),
                PluginState::Active | PluginState::Running | PluginState::Failed
            ) {
                record.process = None;
                return;
            }
            if record.lifecycle.current_state() != PluginState::Failed {
                let _ = record.lifecycle.transition(PluginState::Deactivating);
            }
            record.process.clone()
        };

        let Some(process) = process else {
            return;
        };
        let result = process.deactivate(self.deactivation_timeout());
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            match result {
                Ok(()) => {
                    let _ = record.lifecycle.transition(PluginState::Terminated);
                }
                Err(error) => {
                    record.lifecycle.fail(
                        plugin_id,
                        &error.code,
                        error.message,
                        None,
                        process.stderr_snapshot(),
                    );
                }
            }
        }
    }

    fn activation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.activation_timeout_ms)
    }

    fn deactivation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.deactivation_timeout_ms)
    }
}
