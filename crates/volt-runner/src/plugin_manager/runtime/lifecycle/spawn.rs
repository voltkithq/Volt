use std::sync::Arc;

use crate::plugin_manager::runtime::PluginStartupMode;
use crate::plugin_manager::{
    HostIpcSettings, PLUGIN_NOT_AVAILABLE_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginBootstrapConfig,
    PluginLifecycleEvent, PluginManager, PluginProcessHandle, PluginRuntimeError, PluginState,
    now_ms,
};

impl PluginManager {
    pub(in crate::plugin_manager) fn ensure_plugin_loaded(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        self.ensure_plugin_started(plugin_id, PluginStartupMode::LoadOnly, false)
    }

    pub(in crate::plugin_manager) fn ensure_plugin_running(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        self.ensure_plugin_started(plugin_id, PluginStartupMode::Activate, false)
    }

    pub(in crate::plugin_manager) fn retry_failed_plugin(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        self.ensure_plugin_started(plugin_id, PluginStartupMode::Activate, true)
    }

    fn ensure_plugin_started(
        &self,
        plugin_id: &str,
        mode: PluginStartupMode,
        allow_retry_from_failed: bool,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        let spawn_lock = {
            let registry = self
                .inner
                .registry
                .lock()
                .map_err(|_| registry_unavailable())?;
            let record = registry
                .plugins
                .get(plugin_id)
                .ok_or_else(|| unavailable_plugin(plugin_id))?;
            record.spawn_lock.clone()
        };
        let _guard = spawn_lock.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("spawn lock for plugin '{plugin_id}' is poisoned"),
        })?;

        if let Some((process, state)) = self.current_process(plugin_id) {
            if state == PluginState::Loaded && mode == PluginStartupMode::Activate {
                self.activate_loaded_process(plugin_id, process.clone())?;
            } else if state != PluginState::Loaded {
                self.record_activity(plugin_id);
            }
            return Ok(process);
        }

        let (bootstrap, spawn_event) =
            self.prepare_spawn(plugin_id, mode, allow_retry_from_failed)?;
        if let Some(event) = spawn_event {
            self.emit_lifecycle_event(event);
        }
        let process = self.inner.factory.spawn(&bootstrap)?;
        let manager = self.clone();
        let plugin_id_for_exit = plugin_id.to_string();
        process.set_exit_listener(Arc::new(move |exit| {
            manager.handle_process_exit(&plugin_id_for_exit, exit);
        }));
        let manager = self.clone();
        let plugin_id_for_messages = plugin_id.to_string();
        process.set_message_listener(Arc::new(move |message| {
            manager.handle_plugin_message(&plugin_id_for_messages, message)
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
            return Err(unavailable_message(plugin_id, "failed before ready"));
        }
        self.transition_plugin(plugin_id, PluginState::Loaded)?;
        if mode == PluginStartupMode::LoadOnly {
            return Ok(process);
        }

        self.activate_loaded_process(plugin_id, process.clone())?;
        Ok(process)
    }

    fn activate_loaded_process(
        &self,
        plugin_id: &str,
        process: Arc<dyn PluginProcessHandle>,
    ) -> Result<(), PluginRuntimeError> {
        if let Err(error) = process.activate(self.activation_timeout()) {
            self.fail_plugin(
                plugin_id,
                &error.code,
                error.message,
                None,
                process.stderr_snapshot(),
            );
            let _ = process.kill();
            return Err(unavailable_message(plugin_id, "failed to activate"));
        }
        self.transition_plugin(plugin_id, PluginState::Active)?;
        self.transition_plugin(plugin_id, PluginState::Running)?;
        self.record_activity(plugin_id);
        self.start_watchdog(plugin_id.to_string(), process.clone());
        Ok(())
    }

    fn current_process(
        &self,
        plugin_id: &str,
    ) -> Option<(Arc<dyn PluginProcessHandle>, PluginState)> {
        let Ok(registry) = self.inner.registry.lock() else {
            return None;
        };
        let record = registry.plugins.get(plugin_id)?;
        let state = record.lifecycle.current_state();
        matches!(
            state,
            PluginState::Loaded | PluginState::Active | PluginState::Running
        )
        .then(|| record.process.clone().map(|process| (process, state)))
        .flatten()
    }

    fn prepare_spawn(
        &self,
        plugin_id: &str,
        mode: PluginStartupMode,
        allow_retry_from_failed: bool,
    ) -> Result<(PluginBootstrapConfig, Option<PluginLifecycleEvent>), PluginRuntimeError> {
        let mut registry = self
            .inner
            .registry
            .lock()
            .map_err(|_| registry_unavailable())?;
        let current_state = registry
            .plugins
            .get(plugin_id)
            .map(|record| record.lifecycle.current_state())
            .ok_or_else(|| unavailable_plugin(plugin_id))?;
        let spawn_event =
            match current_state {
                PluginState::Validated | PluginState::Terminated => Some(
                    self.transition_plugin_locked(&mut registry, plugin_id, PluginState::Spawning)?,
                ),
                PluginState::Failed if allow_retry_from_failed => Some(
                    self.transition_plugin_locked(&mut registry, plugin_id, PluginState::Spawning)?,
                ),
                PluginState::Loaded if mode == PluginStartupMode::Activate => None,
                PluginState::Active | PluginState::Running => None,
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
            };

        let record = registry
            .plugins
            .get(plugin_id)
            .expect("plugin exists after state transition");
        let data_root = record.data_root.clone().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("plugin '{plugin_id}' is missing a data root"),
        })?;
        Ok((
            PluginBootstrapConfig {
                plugin_id: record.manifest.id.clone(),
                backend_entry: record.manifest.backend_entry.display().to_string(),
                manifest: record.manifest.raw_manifest.clone(),
                capabilities: record.effective_capabilities.iter().cloned().collect(),
                data_root: data_root.display().to_string(),
                delegated_grants: record
                    .delegated_grants
                    .iter()
                    .filter_map(|grant_id| {
                        volt_core::grant_store::resolve_grant(grant_id)
                            .ok()
                            .map(|path| crate::plugin_manager::DelegatedGrant {
                                grant_id: grant_id.clone(),
                                path: path.display().to_string(),
                            })
                    })
                    .collect(),
                host_ipc_settings: HostIpcSettings {
                    heartbeat_interval_ms: self.inner.config.limits.heartbeat_interval_ms,
                    heartbeat_timeout_ms: self.inner.config.limits.heartbeat_timeout_ms,
                    call_timeout_ms: self.inner.config.limits.call_timeout_ms,
                    max_inflight: 64,
                    max_queue_depth: 256,
                },
            },
            spawn_event,
        ))
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
}

fn registry_unavailable() -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
        message: "plugin registry is unavailable".to_string(),
    }
}

fn unavailable_plugin(plugin_id: &str) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
        message: format!("plugin '{plugin_id}' is not registered"),
    }
}

fn unavailable_message(plugin_id: &str, reason: &str) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
        message: format!("plugin '{plugin_id}' {reason}"),
    }
}
