use std::sync::Arc;

use super::super::*;
use super::access_support::FakeAccessPicker;
use super::process_support::FakeProcessFactory;
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

pub(super) fn manager_with_factory(
    config: RunnerPluginConfig,
    factory: Arc<dyn PluginProcessFactory>,
) -> PluginManager {
    manager_with_picker(config, factory, Arc::new(FakeAccessPicker::default()))
}

pub(super) fn manager_with_picker(
    config: RunnerPluginConfig,
    factory: Arc<dyn PluginProcessFactory>,
    picker: Arc<dyn PluginAccessPicker>,
) -> PluginManager {
    PluginManager::with_dependencies(
        "Volt Test".to_string(),
        &[
            "fs".to_string(),
            "http".to_string(),
            "secureStorage".to_string(),
        ],
        config,
        factory,
        picker,
    )
    .expect("manager")
}

#[allow(dead_code)]
pub(super) fn factory_from_empty() -> Arc<FakeProcessFactory> {
    Arc::new(FakeProcessFactory::new(std::collections::HashMap::new()))
}

pub(super) fn register_ipc_handler(manager: &PluginManager, plugin_id: &str, channel: &str) {
    let response = manager.handle_plugin_message(
        plugin_id,
        WireMessage {
            message_type: WireMessageType::Request,
            id: "register-ipc".to_string(),
            method: "plugin:register-ipc".to_string(),
            payload: Some(serde_json::json!({ "channel": channel })),
            error: None,
        },
    );
    assert!(response.is_some());
}
