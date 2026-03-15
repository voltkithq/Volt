use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use volt_core::ipc::IpcRequest;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::{manager_with_factory, register_ipc_handler};
use crate::runner::config::RunnerPluginConfig;

fn build_manager_with_root(name: &str) -> (TempDir, PluginManager) {
    let root = TempDir::new(name);
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::from([(
            "acme.search".to_string(),
            FakePlan {
                requests: HashMap::from([(
                    "plugin:invoke-ipc".to_string(),
                    FakeRequestOutcome::Success(serde_json::json!({ "ok": true })),
                )]),
                ..FakePlan::default()
            },
        )]))),
    );
    (root, manager)
}

#[test]
fn lifecycle_bus_replays_discovery_transitions_when_rediscovering() {
    let (_root, manager) = build_manager_with_root("lifecycle-bus-redisco");
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_handler = events.clone();
    manager.on_lifecycle(Box::new(move |event| {
        events_for_handler
            .lock()
            .expect("events")
            .push(event.clone());
    }));

    manager.discover_plugins();

    let states = events
        .lock()
        .expect("events")
        .iter()
        .map(|event| event.new_state)
        .collect::<Vec<_>>();
    assert_eq!(
        states,
        vec![PluginState::Discovered, PluginState::Validated]
    );
}

#[test]
fn lifecycle_bus_emits_runtime_transitions_in_order_to_multiple_subscribers() {
    let (_root, manager) = build_manager_with_root("lifecycle-bus-runtime");
    register_ipc_handler(&manager, "acme.search", "ping");
    let first = Arc::new(Mutex::new(Vec::new()));
    let second = Arc::new(Mutex::new(Vec::new()));
    let first_for_handler = first.clone();
    let second_for_handler = second.clone();
    manager.on_lifecycle(Box::new(move |event| {
        first_for_handler
            .lock()
            .expect("first")
            .push(event.new_state);
    }));
    manager.on_lifecycle(Box::new(move |event| {
        second_for_handler
            .lock()
            .expect("second")
            .push(event.new_state);
    }));

    let _ = manager.handle_ipc_request(
        &IpcRequest {
            id: "req-1".to_string(),
            method: "plugin:acme.search:ping".to_string(),
            args: Value::Null,
        },
        Duration::from_millis(50),
    );
    manager.shutdown_all();

    let expected = vec![
        PluginState::Spawning,
        PluginState::Loaded,
        PluginState::Active,
        PluginState::Running,
        PluginState::Deactivating,
        PluginState::Terminated,
    ];
    assert_eq!(*first.lock().expect("first"), expected);
    assert_eq!(*second.lock().expect("second"), expected);
}

#[test]
fn failed_and_activated_subscribers_are_filtered_and_off_removes_handlers() {
    let (_root, manager) = build_manager_with_root("lifecycle-bus-filters");
    let failed = Arc::new(Mutex::new(Vec::new()));
    let activated = Arc::new(Mutex::new(Vec::new()));
    let removed = Arc::new(Mutex::new(Vec::new()));
    let failed_for_handler = failed.clone();
    let activated_for_handler = activated.clone();
    let removed_for_handler = removed.clone();
    manager.on_plugin_failed(Box::new(move |event| {
        failed_for_handler
            .lock()
            .expect("failed")
            .push(event.clone());
    }));
    manager.on_plugin_activated(Box::new(move |event| {
        activated_for_handler
            .lock()
            .expect("activated")
            .push(event.new_state);
    }));
    let removed_subscription = manager.on_lifecycle(Box::new(move |event| {
        removed_for_handler
            .lock()
            .expect("removed")
            .push(event.new_state);
    }));
    manager.off(removed_subscription);

    manager.fail_plugin(
        "acme.search",
        "PLUGIN_BROKEN",
        "boom".to_string(),
        Some(serde_json::json!({ "attempt": 1 })),
        Some("stderr".to_string()),
    );
    manager.retry_plugin("acme.search").expect("retry");

    let failed = failed.lock().expect("failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].new_state, PluginState::Failed);
    assert_eq!(
        failed[0].error.as_ref().expect("error").code,
        "PLUGIN_BROKEN"
    );
    assert_eq!(
        failed[0].error.as_ref().expect("error").stderr.as_deref(),
        Some("stderr")
    );
    assert_eq!(
        *activated.lock().expect("activated"),
        vec![PluginState::Active]
    );
    assert!(removed.lock().expect("removed").is_empty());
}
