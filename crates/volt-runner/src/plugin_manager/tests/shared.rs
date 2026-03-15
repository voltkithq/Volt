use std::sync::Arc;

use super::super::*;
use super::process_support::FakeProcessFactory;
use crate::runner::config::RunnerPluginConfig;

pub(super) fn manager_with_factory(
    config: RunnerPluginConfig,
    factory: Arc<dyn PluginProcessFactory>,
) -> PluginManager {
    PluginManager::with_factory(
        "Volt Test".to_string(),
        &[
            "fs".to_string(),
            "http".to_string(),
            "secureStorage".to_string(),
        ],
        config,
        factory,
    )
    .expect("manager")
}

#[allow(dead_code)]
pub(super) fn factory_from_empty() -> Arc<FakeProcessFactory> {
    Arc::new(FakeProcessFactory::new(std::collections::HashMap::new()))
}
