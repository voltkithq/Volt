use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use super::*;
use crate::runner::config::{RunnerPluginConfig, RunnerPluginLimits, RunnerPluginSpawning};

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("volt-plugin-manager-{name}-{}", now_ms()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Clone)]
struct FakeProcessFactory {
    plans: Arc<Mutex<HashMap<String, FakePlan>>>,
    spawn_count: Arc<AtomicU64>,
}

#[derive(Clone, Default)]
struct FakePlan {
    ready: FakeOutcome,
    activate: FakeOutcome,
    heartbeats: Vec<FakeOutcome>,
    requests: HashMap<String, FakeRequestOutcome>,
    deactivate: FakeOutcome,
    killed: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
enum FakeOutcome {
    #[default]
    Ok,
    Timeout,
    Error(&'static str),
    Crash(i32),
}

#[derive(Clone)]
enum FakeRequestOutcome {
    Success(Value),
    Error(&'static str, &'static str),
    Timeout,
    Crash(i32),
}

impl FakeProcessFactory {
    fn new(plans: HashMap<String, FakePlan>) -> Self {
        Self {
            plans: Arc::new(Mutex::new(plans)),
            spawn_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl PluginProcessFactory for FakeProcessFactory {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        self.spawn_count.fetch_add(1, Ordering::Relaxed);
        let plan = self
            .plans
            .lock()
            .expect("plans")
            .get(&config.plugin_id)
            .cloned()
            .unwrap_or_default();
        Ok(Arc::new(FakeProcessHandle::new(plan)))
    }
}

struct FakeProcessHandle {
    plan: Mutex<FakePlan>,
    exit_listener: Mutex<Option<ExitListener>>,
}

impl FakeProcessHandle {
    fn new(plan: FakePlan) -> Self {
        Self {
            plan: Mutex::new(plan),
            exit_listener: Mutex::new(None),
        }
    }

    fn notify_exit(&self, code: i32) {
        if let Some(listener) = self.exit_listener.lock().expect("listener").clone() {
            listener(ProcessExitInfo { code: Some(code) });
        }
    }
}

impl PluginProcessHandle for FakeProcessHandle {
    fn process_id(&self) -> Option<u32> {
        Some(42)
    }

    fn wait_for_ready(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").ready.clone() {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "ready timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn activate(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").activate.clone() {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "activate timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn request(
        &self,
        method: &str,
        _payload: Value,
        _timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError> {
        let outcome = self
            .plan
            .lock()
            .expect("plan")
            .requests
            .get(method)
            .cloned()
            .unwrap_or(FakeRequestOutcome::Error("UNHANDLED", "no handler"));
        match outcome {
            FakeRequestOutcome::Success(payload) => Ok(WireMessage {
                message_type: WireMessageType::Response,
                id: "response".to_string(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            }),
            FakeRequestOutcome::Error(code, message) => Ok(WireMessage {
                message_type: WireMessageType::Response,
                id: "response".to_string(),
                method: method.to_string(),
                payload: None,
                error: Some(WireError {
                    code: code.to_string(),
                    message: message.to_string(),
                }),
            }),
            FakeRequestOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "call timeout".to_string(),
            }),
            FakeRequestOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn heartbeat(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        let outcome = {
            let mut plan = self.plan.lock().expect("plan");
            if plan.heartbeats.is_empty() {
                FakeOutcome::Ok
            } else {
                plan.heartbeats.remove(0)
            }
        };
        match outcome {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: PLUGIN_HEARTBEAT_TIMEOUT_CODE.to_string(),
                message: "heartbeat timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn deactivate(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").deactivate.clone() {
            FakeOutcome::Ok => {
                self.notify_exit(0);
                Ok(())
            }
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "deactivate timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn kill(&self) -> Result<(), PluginRuntimeError> {
        let killed = self.plan.lock().expect("plan").killed.clone();
        killed.store(true, Ordering::Relaxed);
        self.notify_exit(-1);
        Ok(())
    }

    fn set_exit_listener(&self, listener: Arc<dyn Fn(ProcessExitInfo) + Send + Sync>) {
        *self.exit_listener.lock().expect("listener") = Some(listener);
    }

    fn stderr_snapshot(&self) -> Option<String> {
        None
    }
}

fn write_manifest(path: &Path, id: &str, capabilities: &[&str]) {
    let manifest = serde_json::json!({
        "id": id,
        "name": "Test Plugin",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": capabilities
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
    fs::write(path, serde_json::to_vec(&manifest).expect("manifest json")).expect("manifest");
    let backend = path
        .parent()
        .expect("manifest parent")
        .join("dist")
        .join("plugin.js");
    fs::create_dir_all(backend.parent().expect("backend parent")).expect("backend dir");
    fs::write(backend, b"export default {};\n").expect("backend");
}

fn manager_with_factory(
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

#[test]
fn discovery_finds_manifests_and_reports_missing_directories() {
    let root = TempDir::new("discovery");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![
                root.join("plugins").display().to_string(),
                root.join("missing").display().to_string(),
            ],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .state,
        PluginState::Validated
    );
    assert_eq!(manager.discovery_issues().len(), 1);
}

#[test]
fn discovery_reports_invalid_manifest_without_registering_plugin() {
    let root = TempDir::new("invalid-manifest");
    let manifest_path = root.join("plugins/acme.broken/volt-plugin.json");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("manifest dir");
    fs::write(
        &manifest_path,
        br#"{
            "id": "acme.broken",
            "name": "Broken Plugin",
            "version": "0.1.0",
            "apiVersion": 1,
            "engine": { "volt": "^0.1.0" },
            "backend": "./dist/missing.js",
            "capabilities": ["fs"]
        }"#,
    )
    .expect("manifest");
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.broken".to_string()],
            grants: BTreeMap::from([("acme.broken".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    assert!(manager.get_plugin_state("acme.broken").is_none());
    let issues = manager.discovery_issues();
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].path.as_ref(), Some(&manifest_path));
    assert!(issues[0].message.contains("does not exist"));
    assert!(issues[1].message.contains("enabled plugin 'acme.broken'"));
}

#[test]
fn capability_intersection_rejects_unsatisfied_plugins_and_keeps_exact_matches() {
    let root = TempDir::new("capabilities");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs", "http"],
    );
    write_manifest(
        &root.join("plugins/acme.clip/volt-plugin.json"),
        "acme.clip",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string(), "acme.clip".to_string()],
            grants: BTreeMap::from([
                ("acme.search".to_string(), vec!["fs".to_string()]),
                (
                    "acme.clip".to_string(),
                    vec!["fs".to_string(), "http".to_string()],
                ),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    let failed = manager.get_plugin_state("acme.search").expect("search");
    assert_eq!(failed.state, PluginState::Failed);
    assert_eq!(failed.plugin_id, "acme.search");
    assert!(failed.enabled);
    assert!(
        failed
            .manifest_path
            .ends_with(Path::new("acme.search").join("volt-plugin.json"))
    );
    assert_eq!(
        failed.requested_capabilities,
        vec!["fs".to_string(), "http".to_string()]
    );
    assert_eq!(failed.effective_capabilities, vec!["fs".to_string()]);
    assert!(failed.data_root.as_ref().expect("data root").exists());
    assert_eq!(failed.transitions.len(), 2);
    assert_eq!(failed.transitions[0].new_state, PluginState::Discovered);
    assert_eq!(failed.transitions[1].new_state, PluginState::Failed);
    assert_eq!(failed.errors.len(), 1);
    assert_eq!(failed.errors[0].plugin_id, "acme.search");
    assert_eq!(failed.errors[0].state, PluginState::Failed);
    assert_eq!(failed.errors[0].code, PLUGIN_NOT_AVAILABLE_CODE);
    assert!(failed.errors[0].message.contains("unsatisfiable"));
    assert!(failed.errors[0].details.is_some());
    assert!(failed.errors[0].stderr.is_none());
    assert!(failed.errors[0].timestamp_ms > 0);
    assert_eq!(failed.metrics.pid, None);
    assert_eq!(failed.metrics.missed_heartbeats, 0);
    assert!(!failed.process_running);

    let exact = manager.get_plugin_state("acme.clip").expect("clip");
    assert_eq!(exact.state, PluginState::Validated);
    assert_eq!(exact.effective_capabilities, vec!["fs".to_string()]);
}

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
                "ping".to_string(),
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
    assert_eq!(snapshot.state, PluginState::Running);
    assert_eq!(snapshot.metrics.pid, Some(42));
    assert!(snapshot.metrics.started_at_ms.is_some());
    assert!(snapshot.metrics.last_activity_ms.is_some());
    assert!(snapshot.process_running);
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
            .state,
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
    assert_eq!(snapshot.state, PluginState::Failed);
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
    assert_eq!(snapshot.state, PluginState::Failed);
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
            .state,
        PluginState::Running
    );
}

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
                requests: HashMap::from([("ping".to_string(), FakeRequestOutcome::Timeout)]),
                ..FakePlan::default()
            },
        )]))),
    );

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
            .state,
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
                requests: HashMap::from([("ping".to_string(), FakeRequestOutcome::Crash(17))]),
                ..FakePlan::default()
            },
        )]))),
    );

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
    assert_eq!(snapshot.state, PluginState::Failed);
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

    let _ = manager.handle_ipc_request(
        &IpcRequest {
            id: "req-1".to_string(),
            method: "plugin:acme.search:ping".to_string(),
            args: Value::Null,
        },
        Duration::from_millis(50),
    );
    thread::sleep(Duration::from_millis(60));

    assert!(killed.load(Ordering::Relaxed));
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .state,
        PluginState::Failed
    );
}

#[test]
fn boot_rule_validation_does_not_spawn_plugin_processes() {
    let root = TempDir::new("boot");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let factory = Arc::new(FakeProcessFactory::new(HashMap::new()));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );

    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 0);
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .state,
        PluginState::Validated
    );
    assert!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .data_root
            .expect("data root")
            .exists()
    );
}
