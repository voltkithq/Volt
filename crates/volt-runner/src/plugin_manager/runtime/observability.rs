use crate::plugin_manager::{
    PluginError, PluginLifecycleEvent, PluginManager, PluginRecord, PluginStateSnapshot,
    SubscriptionId,
};

impl PluginManager {
    pub(crate) fn on_lifecycle(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.inner.lifecycle_bus.on_lifecycle(handler)
    }

    pub(crate) fn on_plugin_failed(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.inner.lifecycle_bus.on_plugin_failed(handler)
    }

    pub(crate) fn on_plugin_activated(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.inner.lifecycle_bus.on_plugin_activated(handler)
    }

    #[allow(dead_code)]
    pub(crate) fn off(&self, subscription_id: SubscriptionId) {
        self.inner.lifecycle_bus.off(subscription_id);
    }

    pub(crate) fn get_plugin_state(&self, plugin_id: &str) -> Option<PluginStateSnapshot> {
        let registry = self.inner.registry.lock().ok()?;
        let record = registry.plugins.get(plugin_id)?;
        Some(record.snapshot())
    }

    pub(crate) fn get_states(&self) -> Vec<PluginStateSnapshot> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut states = registry
            .plugins
            .values()
            .map(PluginRecord::snapshot)
            .collect::<Vec<_>>();
        states.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
        states
    }

    pub(crate) fn get_errors(&self) -> Vec<PluginError> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut errors = registry
            .plugins
            .values()
            .flat_map(|record| record.lifecycle.errors.clone())
            .collect::<Vec<_>>();
        errors.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
        errors
    }

    pub(crate) fn get_plugin_errors(&self, plugin_id: &str) -> Vec<PluginError> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let Some(record) = registry.plugins.get(plugin_id) else {
            return Vec::new();
        };
        let mut errors = record.lifecycle.errors.clone();
        errors.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
        errors
    }
}
