use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::Value;
use volt_core::ipc::IpcRequest;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakeOutcome, FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::{manager_with_factory, register_ipc_handler};
use crate::runner::config::{RunnerPluginConfig, RunnerPluginSpawning};

#[test]
fn lazy_spawn_happens_on_first_ipc_request() {
    let root = TempDir::new("lazy");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let factory = Arc::new(FakeProcessFactory::new(HashMap::from([(
        "acme.search".to_string(),
        FakePlan {
            requests: HashMap::from([(
                "plugin:invoke-ipc".to_string(),
                FakeRequestOutcome::Success(serde_json::json!({ "ok": true })),
            )]),
            ..FakePlan::default()
        },
    )])));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );
    register_ipc_handler(&manager, "acme.search", "ping");

    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 0);
    let response = manager
        .handle_ipc_request(
            &IpcRequest {
                id: "req-1".to_string(),
                method: "plugin:acme.search:ping".to_string(),
                args: Value::Null,
            },
            Duration::from_millis(100),
        )
        .expect("response");

    assert_eq!(response.result, Some(serde_json::json!({ "ok": true })));
    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 1);
    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.current_state, PluginState::Running);
    assert_eq!(snapshot.metrics.pid, Some(42));
    assert!(snapshot.metrics.started_at_ms.is_some());
    assert!(snapshot.metrics.last_activity_ms.is_some());
    assert!(snapshot.process_running);
}

#[test]
fn spawn_timeout_moves_plugin_to_failed() {
    let root = TempDir::new("timeout");
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
                ready: FakeOutcome::Timeout,
                ..FakePlan::default()
            },
        )]))),
    );
    register_ipc_handler(&manager, "acme.search", "ping");

    let response = manager
        .handle_ipc_request(
            &IpcRequest {
                id: "req-1".to_string(),
                method: "plugin:acme.search:ping".to_string(),
                args: Value::Null,
            },
            Duration::from_millis(50),
        )
        .expect("response");

    assert_eq!(
        response.error_code.as_deref(),
        Some(PLUGIN_NOT_AVAILABLE_CODE)
    );
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .current_state,
        PluginState::Failed
    );
    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert!(snapshot.errors.iter().any(|error| error.code == "TIMEOUT"));
}

#[test]
fn spawn_crash_moves_plugin_to_failed() {
    let root = TempDir::new("spawn-crash");
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
                ready: FakeOutcome::Crash(9),
                ..FakePlan::default()
            },
        )]))),
    );
    register_ipc_handler(&manager, "acme.search", "ping");

    let response = manager
        .handle_ipc_request(
            &IpcRequest {
                id: "req-1".to_string(),
                method: "plugin:acme.search:ping".to_string(),
                args: Value::Null,
            },
            Duration::from_millis(50),
        )
        .expect("response");

    assert_eq!(
        response.error_code.as_deref(),
        Some(PLUGIN_NOT_AVAILABLE_CODE)
    );
    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.current_state, PluginState::Failed);
    assert!(snapshot.errors.iter().any(|error| {
        error
            .details
            .as_ref()
            .and_then(|details| details.get("exitCode"))
            == Some(&serde_json::json!(9))
    }));
}

#[test]
fn activation_error_moves_plugin_to_failed() {
    let root = TempDir::new("activate-error");
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
                activate: FakeOutcome::Error("activate failed"),
                ..FakePlan::default()
            },
        )]))),
    );
    register_ipc_handler(&manager, "acme.search", "ping");

    let response = manager
        .handle_ipc_request(
            &IpcRequest {
                id: "req-1".to_string(),
                method: "plugin:acme.search:ping".to_string(),
                args: Value::Null,
            },
            Duration::from_millis(50),
        )
        .expect("response");

    assert_eq!(
        response.error_code.as_deref(),
        Some(PLUGIN_NOT_AVAILABLE_CODE)
    );
    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.current_state, PluginState::Failed);
    assert!(
        snapshot
            .errors
            .iter()
            .any(|error| error.message.contains("activate failed"))
    );
}

#[test]
fn pre_spawn_forces_startup_activation_after_window_ready_hook() {
    let root = TempDir::new("prespawn");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let factory = Arc::new(FakeProcessFactory::new(HashMap::from([(
        "acme.search".to_string(),
        FakePlan::default(),
    )])));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            spawning: RunnerPluginSpawning {
                pre_spawn: vec!["acme.search".to_string()],
                ..RunnerPluginSpawning::default()
            },
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );

    manager.run_pre_spawn_now();
    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 1);
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .current_state,
        PluginState::Running
    );
}
