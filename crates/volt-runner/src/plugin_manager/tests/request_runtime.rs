use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use volt_core::ipc::IpcRequest;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakeOutcome, FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::{manager_with_factory, register_ipc_handler};
use crate::runner::config::{RunnerPluginConfig, RunnerPluginLimits};

#[test]
fn request_timeout_maps_to_ipc_timeout_error_without_crashing_plugin() {
    let root = TempDir::new("request-timeout");
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
                    FakeRequestOutcome::Timeout,
                )]),
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
        Some(IPC_HANDLER_TIMEOUT_CODE)
    );
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .current_state,
        PluginState::Running
    );
}

#[test]
fn request_crash_transitions_running_plugin_to_failed() {
    let root = TempDir::new("request-crash");
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
                    FakeRequestOutcome::Crash(17),
                )]),
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
        Some(PLUGIN_RUNTIME_ERROR_CODE)
    );
    thread::sleep(Duration::from_millis(20));
    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.current_state, PluginState::Failed);
    assert!(snapshot.errors.iter().any(|error| {
        error
            .details
            .as_ref()
            .and_then(|details| details.get("exitCode"))
            == Some(&serde_json::json!(17))
    }));
}

#[test]
fn watchdog_kills_after_two_missed_heartbeats() {
    let root = TempDir::new("watchdog");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let killed = Arc::new(AtomicBool::new(false));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            limits: RunnerPluginLimits {
                heartbeat_interval_ms: 10,
                heartbeat_timeout_ms: 10,
                ..RunnerPluginLimits::default()
            },
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::from([(
            "acme.search".to_string(),
            FakePlan {
                heartbeats: vec![FakeOutcome::Timeout, FakeOutcome::Timeout],
                killed: killed.clone(),
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
    let mut snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    for _ in 0..50 {
        if killed.load(Ordering::Relaxed) && snapshot.current_state == PluginState::Failed {
            break;
        }
        thread::sleep(Duration::from_millis(10));
        snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    }

    assert!(killed.load(Ordering::Relaxed));
    assert_eq!(snapshot.current_state, PluginState::Failed);
}
