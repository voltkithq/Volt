use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::{
    PLUGIN_HEARTBEAT_TIMEOUT_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginManager, PluginProcessHandle,
    PluginState, ProcessExitInfo, now_ms,
};

impl PluginManager {
    pub(super) fn handle_process_exit(&self, plugin_id: &str, exit: ProcessExitInfo) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            match record.lifecycle.current_state() {
                PluginState::Deactivating | PluginState::Terminated | PluginState::Disabled => {
                    let _ = record.lifecycle.transition(PluginState::Terminated);
                }
                PluginState::Failed => {}
                _ => record.lifecycle.fail(
                    plugin_id,
                    PLUGIN_RUNTIME_ERROR_CODE,
                    format!(
                        "plugin process exited unexpectedly with code {:?}",
                        exit.code
                    ),
                    Some(serde_json::json!({ "exitCode": exit.code })),
                    None,
                ),
            }
        }
    }

    pub(super) fn start_watchdog(&self, plugin_id: String, process: Arc<dyn PluginProcessHandle>) {
        let manager = self.clone();
        let idle_timeout = Duration::from_millis(self.inner.config.spawning.idle_timeout_ms);
        let heartbeat_interval =
            Duration::from_millis(self.inner.config.limits.heartbeat_interval_ms);
        let heartbeat_timeout =
            Duration::from_millis(self.inner.config.limits.heartbeat_timeout_ms);

        let _ = thread::Builder::new()
            .name(format!("volt-plugin-watchdog-{plugin_id}"))
            .spawn(move || {
                let mut missed = 0_u32;
                loop {
                    thread::sleep(heartbeat_interval);

                    let should_stop = {
                        let Ok(registry) = manager.inner.registry.lock() else {
                            return;
                        };
                        let Some(record) = registry.plugins.get(&plugin_id) else {
                            return;
                        };
                        !matches!(
                            record.lifecycle.current_state(),
                            PluginState::Active | PluginState::Running
                        ) || record
                            .process
                            .as_ref()
                            .map(|current| !Arc::ptr_eq(current, &process))
                            .unwrap_or(true)
                    };
                    if should_stop {
                        return;
                    }

                    if idle_timeout.as_millis() > 0 {
                        let should_idle_shutdown = {
                            let Ok(registry) = manager.inner.registry.lock() else {
                                return;
                            };
                            let Some(record) = registry.plugins.get(&plugin_id) else {
                                return;
                            };
                            record.pending_requests == 0
                                && record
                                    .metrics
                                    .last_activity_ms
                                    .map(|last_activity_ms| {
                                        now_ms().saturating_sub(last_activity_ms)
                                            >= idle_timeout.as_millis() as u64
                                    })
                                    .unwrap_or(false)
                        };
                        if should_idle_shutdown {
                            manager.deactivate_plugin(&plugin_id);
                            return;
                        }
                    }

                    if let Ok(mut registry) = manager.inner.registry.lock()
                        && let Some(record) = registry.plugins.get_mut(&plugin_id)
                    {
                        record.metrics.last_heartbeat_sent_ms = Some(now_ms());
                    }

                    match process.heartbeat(heartbeat_timeout) {
                        Ok(()) => {
                            missed = 0;
                            if let Ok(mut registry) = manager.inner.registry.lock()
                                && let Some(record) = registry.plugins.get_mut(&plugin_id)
                            {
                                record.metrics.last_heartbeat_ack_ms = Some(now_ms());
                                record.metrics.missed_heartbeats = 0;
                            }
                        }
                        Err(error) => {
                            missed += 1;
                            if let Ok(mut registry) = manager.inner.registry.lock()
                                && let Some(record) = registry.plugins.get_mut(&plugin_id)
                            {
                                record.metrics.heartbeat_failures += 1;
                                record.metrics.missed_heartbeats = missed;
                            }
                            if missed >= 2 {
                                manager.fail_plugin(
                                    &plugin_id,
                                    PLUGIN_HEARTBEAT_TIMEOUT_CODE,
                                    error.message,
                                    None,
                                    process.stderr_snapshot(),
                                );
                                let _ = process.kill();
                                return;
                            }
                        }
                    }
                }
            });
    }
}
