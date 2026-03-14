use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use volt_core::ipc::IpcRequest;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::manager_with_factory;
use crate::runner::config::RunnerPluginConfig;

#[test]
fn state_machine_rejects_invalid_transitions() {
    let mut lifecycle = PluginLifecycle::new();
    lifecycle
        .transition(PluginState::Discovered)
        .expect("discovered");
    lifecycle
        .transition(PluginState::Validated)
        .expect("validated");
    lifecycle
        .transition(PluginState::Spawning)
        .expect("spawning");
    lifecycle.transition(PluginState::Loaded).expect("loaded");
    lifecycle.transition(PluginState::Active).expect("active");
    lifecycle.transition(PluginState::Running).expect("running");
    lifecycle
        .transition(PluginState::Deactivating)
        .expect("deactivating");
    lifecycle
        .transition(PluginState::Terminated)
        .expect("terminated");

    assert!(lifecycle.transition(PluginState::Loaded).is_err());
}

#[test]
fn get_states_returns_sorted_plugin_snapshots() {
    let root = TempDir::new("states");
    write_manifest(
        &root.join("plugins/acme.zed/volt-plugin.json"),
        "acme.zed",
        &["fs"],
    );
    write_manifest(
        &root.join("plugins/acme.alpha/volt-plugin.json"),
        "acme.alpha",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.zed".to_string(), "acme.alpha".to_string()],
            grants: BTreeMap::from([
                ("acme.zed".to_string(), vec!["fs".to_string()]),
                ("acme.alpha".to_string(), vec!["fs".to_string()]),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    let states = manager.get_states();
    assert_eq!(states.len(), 2);
    assert_eq!(states[0].plugin_id, "acme.alpha");
    assert_eq!(states[1].plugin_id, "acme.zed");
}

#[test]
fn shutdown_all_deactivates_running_plugins_cleanly() {
    let root = TempDir::new("shutdown");
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
                    "ping".to_string(),
                    FakeRequestOutcome::Success(serde_json::json!({ "ok": true })),
                )]),
                ..FakePlan::default()
            },
        )]))),
    );

    let _ = manager.handle_ipc_request(
        &IpcRequest {
            id: "req-1".to_string(),
            method: "plugin:acme.search:ping".to_string(),
            args: Value::Null,
        },
        Duration::from_millis(50),
    );

    manager.shutdown_all();

    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.state, PluginState::Terminated);
    assert!(!snapshot.process_running);
}
