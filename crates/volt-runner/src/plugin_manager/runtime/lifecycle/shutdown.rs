use crate::plugin_manager::{PluginManager, PluginState};

impl PluginManager {
    pub(in crate::plugin_manager) fn deactivate_plugin(&self, plugin_id: &str) {
        let (process, state) = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            let Some(state) = registry
                .plugins
                .get(plugin_id)
                .map(|record| record.lifecycle.current_state())
            else {
                return;
            };
            if !matches!(
                state,
                PluginState::Loaded
                    | PluginState::Active
                    | PluginState::Running
                    | PluginState::Failed
            ) {
                crate::plugin_manager::host_api_helpers::clear_plugin_registrations_locked(
                    &mut registry,
                    plugin_id,
                );
                if let Some(record) = registry.plugins.get_mut(plugin_id) {
                    record.process = None;
                }
                return;
            }
            let record = registry.plugins.get_mut(plugin_id).expect("checked above");
            if matches!(state, PluginState::Active | PluginState::Running) {
                let _ = record.lifecycle.transition(PluginState::Deactivating);
            }
            (record.process.clone(), state)
        };

        let Some(process) = process else {
            return;
        };
        let result = if state == PluginState::Loaded {
            process.kill()
        } else {
            process.deactivate(self.deactivation_timeout())
        };
        if let Ok(mut registry) = self.inner.registry.lock() {
            crate::plugin_manager::host_api_helpers::clear_plugin_registrations_locked(
                &mut registry,
                plugin_id,
            );
            if let Some(record) = registry.plugins.get_mut(plugin_id) {
                record.process = None;
                record.pending_requests = 0;
                match result {
                    Ok(()) => {
                        if state == PluginState::Loaded {
                            record.metrics.pid = None;
                            let _ = record.lifecycle.transition(PluginState::Terminated);
                        } else {
                            let _ = record.lifecycle.transition(PluginState::Terminated);
                        }
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
    }
}
