use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use volt_core::grant_store;
use volt_core::ipc::IpcRequest;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::{lock_grant_state, manager_with_factory, register_ipc_handler};
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
fn state_machine_only_allows_terminated_self_transition() {
    let mut lifecycle = PluginLifecycle::new();
    lifecycle
        .transition(PluginState::Discovered)
        .expect("discovered");
    lifecycle
        .transition(PluginState::Validated)
        .expect("validated");
    assert!(lifecycle.transition(PluginState::Validated).is_err());

    lifecycle
        .transition(PluginState::Spawning)
        .expect("spawning");
    lifecycle.transition(PluginState::Loaded).expect("loaded");
    lifecycle
        .transition(PluginState::Terminated)
        .expect("terminated");
    lifecycle
        .transition(PluginState::Terminated)
        .expect("terminated self-transition");
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
                    "plugin:invoke-ipc".to_string(),
                    FakeRequestOutcome::Success(serde_json::json!({ "ok": true })),
                )]),
                ..FakePlan::default()
            },
        )]))),
    );
    register_ipc_handler(&manager, "acme.search", "ping");

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
    assert_eq!(snapshot.current_state, PluginState::Terminated);
    assert!(!snapshot.process_running);
}

#[test]
fn state_snapshot_reports_active_registrations_and_grants() {
    let _guard = lock_grant_state();
    let root = TempDir::new("snapshot-counts");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let grant_path = root.join("granted");
    std::fs::create_dir_all(&grant_path).expect("grant path");
    let grant_id = grant_store::create_grant(grant_path).expect("grant");
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    manager
        .delegate_grant("acme.search", &grant_id)
        .expect("delegate grant");
    let _ = manager.handle_plugin_message(
        "acme.search",
        crate::plugin_manager::process::WireMessage {
            message_type: WireMessageType::Request,
            id: "register-command".to_string(),
            method: "plugin:register-command".to_string(),
            payload: Some(serde_json::json!({ "id": "reindex" })),
            error: None,
        },
    );
    let _ = manager.handle_plugin_message(
        "acme.search",
        crate::plugin_manager::process::WireMessage {
            message_type: WireMessageType::Request,
            id: "subscribe-event".to_string(),
            method: "plugin:subscribe-event".to_string(),
            payload: Some(serde_json::json!({ "event": "app:focus" })),
            error: None,
        },
    );
    register_ipc_handler(&manager, "acme.search", "ping");

    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.active_registrations.command_count, 1);
    assert_eq!(snapshot.active_registrations.event_subscription_count, 1);
    assert_eq!(snapshot.active_registrations.ipc_handler_count, 1);
    assert_eq!(snapshot.delegated_grant_count, 1);
}

#[test]
fn disabled_plugins_ignore_process_exit_transitions() {
    let root = TempDir::new("disabled-exit");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: Vec::new(),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    manager.handle_process_exit("acme.search", ProcessExitInfo { code: Some(0) });

    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .current_state,
        PluginState::Disabled
    );
}
