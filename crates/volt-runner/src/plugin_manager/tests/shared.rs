use std::sync::{Arc, Mutex, MutexGuard};

use volt_core::{grant_store, plugin_grant_registry};

use super::super::*;
use super::access_support::FakeAccessPicker;
use super::process_support::FakeProcessFactory;
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

/// Shared guard for all tests that touch global grant state (`grant_store`,
/// `plugin_grant_registry`). Every test module that calls `clear_delegations` /
/// `clear_grants` MUST acquire this lock to prevent cross-module interference
/// when `cargo test` runs modules in parallel.
static GRANT_TEST_GUARD: Mutex<()> = Mutex::new(());

pub(super) fn lock_grant_state() -> MutexGuard<'static, ()> {
    let guard = GRANT_TEST_GUARD.lock().expect("grant test guard");
    plugin_grant_registry::clear_delegations();
    grant_store::clear_grants();
    guard
}

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
