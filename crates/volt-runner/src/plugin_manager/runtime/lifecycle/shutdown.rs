use crate::plugin_manager::{PluginManager, PluginState};

impl PluginManager {
    pub(in crate::plugin_manager) fn deactivate_plugin(&self, plugin_id: &str) {
        let (process, state, pre_events) = {
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

            let mut events = Vec::new();
            if matches!(state, PluginState::Active | PluginState::Running)
                && let Ok(event) = self.transition_plugin_locked(
                    &mut registry,
                    plugin_id,
                    PluginState::Deactivating,
                )
            {
                events.push(event);
            }
            let process = registry
                .plugins
                .get(plugin_id)
                .and_then(|record| record.process.clone());
            (process, state, events)
        };
        for event in pre_events {
            self.emit_lifecycle_event(event);
        }

        let Some(process) = process else {
            return;
        };
        let result = if state == PluginState::Loaded {
            process.kill()
        } else {
            process.deactivate(self.deactivation_timeout())
        };

        let post_events = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            crate::plugin_manager::host_api_helpers::clear_plugin_registrations_locked(
                &mut registry,
                plugin_id,
            );
            if let Some(record) = registry.plugins.get_mut(plugin_id) {
                record.process = None;
                record.pending_requests = 0;
            }
            match result {
                Ok(()) => {
                    let already_terminated = registry
                        .plugins
                        .get(plugin_id)
                        .map(|record| record.lifecycle.current_state() == PluginState::Terminated)
                        .unwrap_or(true);
                    if already_terminated {
                        Vec::new()
                    } else {
                        self.transition_plugin_locked(
                            &mut registry,
                            plugin_id,
                            PluginState::Terminated,
                        )
                        .map(|event| vec![event])
                        .unwrap_or_default()
                    }
                }
                Err(error) => self
                    .fail_plugin_locked(
                        &mut registry,
                        plugin_id,
                        &error.code,
                        error.message,
                        None,
                        process.stderr_snapshot(),
                    )
                    .unwrap_or_default(),
            }
        };
        for event in post_events {
            self.emit_lifecycle_event(event);
        }
    }
}
