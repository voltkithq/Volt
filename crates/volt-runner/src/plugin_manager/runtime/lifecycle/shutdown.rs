use crate::plugin_manager::{PluginManager, PluginState};

impl PluginManager {
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
}
