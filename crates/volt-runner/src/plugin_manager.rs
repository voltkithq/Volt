use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use semver::{Version, VersionReq};
use serde_json::Value;
use volt_core::ipc::{IPC_HANDLER_TIMEOUT_CODE, IpcRequest, IpcResponse};
use volt_core::permissions::Permission;

use crate::runner::config::{RunnerPluginConfig, RunnerPluginSpawningStrategy};

const MANIFEST_FILE_NAME: &str = "volt-plugin.json";
const HOST_VOLT_VERSION: &str = env!("CARGO_PKG_VERSION");
const PLUGIN_HOST_PATH_ENV: &str = "VOLT_PLUGIN_HOST_PATH";
const PLUGIN_RUNTIME_ERROR_CODE: &str = "PLUGIN_RUNTIME_ERROR";
const PLUGIN_HEARTBEAT_TIMEOUT_CODE: &str = "PLUGIN_HEARTBEAT_TIMEOUT";
const PLUGIN_NOT_AVAILABLE_CODE: &str = "PLUGIN_NOT_AVAILABLE";
const PLUGIN_ROUTE_INVALID_CODE: &str = "PLUGIN_ROUTE_INVALID";
const SUPPORTED_PLUGIN_API_VERSIONS: &[u64] = &[1];
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
const DEFAULT_EXIT_WAIT_AFTER_KILL_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginState {
    Discovered,
    Validated,
    Spawning,
    Loaded,
    Active,
    Running,
    Deactivating,
    Terminated,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginStateTransition {
    pub(crate) previous_state: Option<PluginState>,
    pub(crate) new_state: PluginState,
    pub(crate) timestamp_ms: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PluginError {
    pub(crate) plugin_id: String,
    pub(crate) state: PluginState,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
    pub(crate) stderr: Option<String>,
    pub(crate) timestamp_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PluginResourceMetrics {
    pub(crate) pid: Option<u32>,
    pub(crate) started_at_ms: Option<u64>,
    pub(crate) last_activity_ms: Option<u64>,
    pub(crate) last_heartbeat_sent_ms: Option<u64>,
    pub(crate) last_heartbeat_ack_ms: Option<u64>,
    pub(crate) missed_heartbeats: u32,
    pub(crate) heartbeat_failures: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct PluginDiscoveryIssue {
    pub(crate) path: Option<PathBuf>,
    pub(crate) message: String,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct PluginSnapshot {
    pub(crate) plugin_id: String,
    pub(crate) state: PluginState,
    pub(crate) enabled: bool,
    pub(crate) manifest_path: PathBuf,
    pub(crate) data_root: Option<PathBuf>,
    pub(crate) requested_capabilities: Vec<String>,
    pub(crate) effective_capabilities: Vec<String>,
    pub(crate) transitions: Vec<PluginStateTransition>,
    pub(crate) errors: Vec<PluginError>,
    pub(crate) metrics: PluginResourceMetrics,
    pub(crate) process_running: bool,
}

#[derive(Clone)]
pub(crate) struct PluginManager {
    inner: Arc<PluginManagerInner>,
}

struct PluginManagerInner {
    config: RunnerPluginConfig,
    app_permissions: HashSet<Permission>,
    app_data_root: PathBuf,
    factory: Arc<dyn PluginProcessFactory>,
    registry: Mutex<PluginRegistry>,
}

#[derive(Default)]
struct PluginRegistry {
    plugins: HashMap<String, PluginRecord>,
    discovery_issues: Vec<PluginDiscoveryIssue>,
}

struct PluginRecord {
    manifest: PluginManifest,
    manifest_path: PathBuf,
    enabled: bool,
    data_root: Option<PathBuf>,
    #[cfg(test)]
    requested_capabilities: BTreeSet<String>,
    effective_capabilities: BTreeSet<String>,
    lifecycle: PluginLifecycle,
    metrics: PluginResourceMetrics,
    process: Option<Arc<dyn PluginProcessHandle>>,
    pending_requests: usize,
    spawn_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
struct PluginLifecycle {
    state: PluginState,
    transitions: Vec<PluginStateTransition>,
    errors: Vec<PluginError>,
}

#[derive(Debug, Clone)]
struct PluginManifest {
    id: String,
    capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
struct PluginRoute {
    plugin_id: String,
    method: String,
}

#[derive(Debug, Clone)]
struct PluginRuntimeError {
    code: String,
    message: String,
}

trait PluginProcessFactory: Send + Sync {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError>;
}

trait PluginProcessHandle: Send + Sync {
    fn process_id(&self) -> Option<u32>;
    fn wait_for_ready(&self, timeout: Duration) -> Result<(), PluginRuntimeError>;
    fn activate(&self, timeout: Duration) -> Result<(), PluginRuntimeError>;
    fn request(
        &self,
        method: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError>;
    fn heartbeat(&self, timeout: Duration) -> Result<(), PluginRuntimeError>;
    fn deactivate(&self, timeout: Duration) -> Result<(), PluginRuntimeError>;
    fn kill(&self) -> Result<(), PluginRuntimeError>;
    fn set_exit_listener(&self, listener: Arc<dyn Fn(ProcessExitInfo) + Send + Sync>);
    fn stderr_snapshot(&self) -> Option<String>;
}

#[derive(Default)]
struct RealPluginProcessFactory;

#[derive(Debug, Clone)]
struct ProcessExitInfo {
    code: Option<i32>,
}

type ExitListener = Arc<dyn Fn(ProcessExitInfo) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireMessageType {
    Request,
    Response,
    Event,
    Signal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WireError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WireMessage {
    #[serde(rename = "type")]
    message_type: WireMessageType,
    id: String,
    method: String,
    payload: Option<Value>,
    error: Option<WireError>,
}

impl WireMessage {
    fn request(id: String, method: impl Into<String>, payload: Value) -> Self {
        Self {
            message_type: WireMessageType::Request,
            id,
            method: method.into(),
            payload: Some(payload),
            error: None,
        }
    }

    fn signal(id: String, method: impl Into<String>, payload: Option<Value>) -> Self {
        Self {
            message_type: WireMessageType::Signal,
            id,
            method: method.into(),
            payload,
            error: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginBootstrapConfig {
    plugin_id: String,
    capabilities: Vec<String>,
    data_root: String,
    delegated_grants: Vec<DelegatedGrant>,
    host_ipc_settings: HostIpcSettings,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DelegatedGrant {
    grant_id: String,
    path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HostIpcSettings {
    heartbeat_interval_ms: u64,
    heartbeat_timeout_ms: u64,
    call_timeout_ms: u64,
    max_inflight: u32,
    max_queue_depth: u32,
}

impl PluginManager {
    pub(crate) fn new(
        app_name: String,
        permissions: &[String],
        config: RunnerPluginConfig,
    ) -> Result<Self, String> {
        Self::with_factory(
            app_name,
            permissions,
            config,
            Arc::new(RealPluginProcessFactory),
        )
    }

    fn with_factory(
        app_name: String,
        permissions: &[String],
        config: RunnerPluginConfig,
        factory: Arc<dyn PluginProcessFactory>,
    ) -> Result<Self, String> {
        let app_permissions = permissions
            .iter()
            .filter_map(|name| Permission::from_str_name(name))
            .collect::<HashSet<_>>();
        let app_data_root = resolve_app_data_root(&app_name)?;
        let manager = Self {
            inner: Arc::new(PluginManagerInner {
                config,
                app_permissions,
                app_data_root,
                factory,
                registry: Mutex::new(PluginRegistry::default()),
            }),
        };
        manager.discover_plugins();
        Ok(manager)
    }

    #[cfg(test)]
    pub(crate) fn get_plugin_state(&self, plugin_id: &str) -> Option<PluginSnapshot> {
        let registry = self.inner.registry.lock().ok()?;
        let record = registry.plugins.get(plugin_id)?;
        Some(record.snapshot())
    }

    #[cfg(test)]
    pub(crate) fn get_states(&self) -> Vec<PluginSnapshot> {
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

    #[cfg(test)]
    pub(crate) fn discovery_issues(&self) -> Vec<PluginDiscoveryIssue> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        registry.discovery_issues.clone()
    }

    pub(crate) fn handle_ipc_request(
        &self,
        request: &IpcRequest,
        timeout: Duration,
    ) -> Option<IpcResponse> {
        match parse_plugin_route(&request.method) {
            Ok(Some(route)) => Some(self.dispatch_plugin_request(request, &route, timeout)),
            Ok(None) => None,
            Err(message) => Some(IpcResponse::error_with_code(
                request.id.clone(),
                message,
                PLUGIN_ROUTE_INVALID_CODE.to_string(),
            )),
        }
    }

    pub(crate) fn start_pre_spawn(&self) {
        let plugin_ids = self.pre_spawn_plugin_ids();
        if plugin_ids.is_empty() {
            return;
        }
        let manager = self.clone();
        let _ = thread::Builder::new()
            .name("volt-plugin-pre-spawn".to_string())
            .spawn(move || {
                for plugin_id in plugin_ids {
                    let _ = manager.ensure_plugin_running(&plugin_id);
                }
            });
    }

    #[cfg(test)]
    fn run_pre_spawn_now(&self) {
        for plugin_id in self.pre_spawn_plugin_ids() {
            let _ = self.ensure_plugin_running(&plugin_id);
        }
    }

    pub(crate) fn shutdown_all(&self) {
        let plugin_ids = {
            let Ok(registry) = self.inner.registry.lock() else {
                return;
            };
            registry.plugins.keys().cloned().collect::<Vec<_>>()
        };
        for plugin_id in plugin_ids {
            self.deactivate_plugin(&plugin_id);
        }
    }

    fn discover_plugins(&self) {
        let enabled = self
            .inner
            .config
            .enabled
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut manifest_paths = Vec::new();
        let mut registry = PluginRegistry::default();

        for directory in &self.inner.config.plugin_dirs {
            let resolved = resolve_plugin_directory(directory);
            if !resolved.exists() {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: Some(resolved),
                    message: format!("plugin directory '{directory}' does not exist"),
                });
                continue;
            }
            if let Err(error) = collect_manifest_paths(&resolved, &mut manifest_paths) {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: Some(resolved),
                    message: format!("failed to scan plugin directory: {error}"),
                });
            }
        }

        manifest_paths.sort();
        let mut discovered_ids = HashSet::new();
        let mut enabled_count = 0_usize;
        for manifest_path in manifest_paths {
            match self.discover_plugin_record(&manifest_path, &enabled) {
                Ok(record) => {
                    if !discovered_ids.insert(record.manifest.id.clone()) {
                        registry.discovery_issues.push(PluginDiscoveryIssue {
                            path: Some(manifest_path),
                            message: format!("duplicate plugin id '{}'", record.manifest.id),
                        });
                        continue;
                    }
                    if record.enabled {
                        enabled_count += 1;
                        if enabled_count > self.inner.config.limits.max_plugins {
                            registry.discovery_issues.push(PluginDiscoveryIssue {
                                path: Some(record.manifest_path.clone()),
                                message: format!(
                                    "max enabled plugins exceeded (limit={})",
                                    self.inner.config.limits.max_plugins
                                ),
                            });
                            continue;
                        }
                    }
                    registry.plugins.insert(record.manifest.id.clone(), record);
                }
                Err(issue) => registry.discovery_issues.push(issue),
            }
        }

        for plugin_id in enabled {
            if !discovered_ids.contains(&plugin_id) {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: None,
                    message: format!(
                        "enabled plugin '{plugin_id}' was not found in configured plugin directories"
                    ),
                });
            }
        }

        if let Ok(mut guard) = self.inner.registry.lock() {
            *guard = registry;
        }
    }

    fn dispatch_plugin_request(
        &self,
        request: &IpcRequest,
        route: &PluginRoute,
        timeout: Duration,
    ) -> IpcResponse {
        let process = match self.ensure_plugin_running(&route.plugin_id) {
            Ok(process) => process,
            Err(error) => {
                return IpcResponse::error_with_details(
                    request.id.clone(),
                    error.message,
                    error.code,
                    serde_json::json!({ "pluginId": route.plugin_id }),
                );
            }
        };

        self.mark_request_started(&route.plugin_id);
        let response = process.request(&route.method, request.args.clone(), timeout);
        self.mark_request_finished(&route.plugin_id);

        match response {
            Ok(message) => {
                self.record_activity(&route.plugin_id);
                if let Some(error) = message.error {
                    IpcResponse::error_with_code(request.id.clone(), error.message, error.code)
                } else {
                    IpcResponse::success(request.id.clone(), message.payload.unwrap_or(Value::Null))
                }
            }
            Err(error) if error.code == "TIMEOUT" => IpcResponse::error_with_details(
                request.id.clone(),
                error.message,
                IPC_HANDLER_TIMEOUT_CODE.to_string(),
                serde_json::json!({
                    "timeoutMs": timeout.as_millis(),
                    "method": request.method
                }),
            ),
            Err(error) => IpcResponse::error_with_details(
                request.id.clone(),
                error.message,
                error.code,
                serde_json::json!({ "pluginId": route.plugin_id }),
            ),
        }
    }

    fn pre_spawn_plugin_ids(&self) -> Vec<String> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut plugin_ids = match self.inner.config.spawning.strategy {
            RunnerPluginSpawningStrategy::Lazy => self.inner.config.spawning.pre_spawn.clone(),
            RunnerPluginSpawningStrategy::Eager => registry
                .plugins
                .values()
                .filter(|record| {
                    record.enabled && record.lifecycle.current_state() == PluginState::Validated
                })
                .map(|record| record.manifest.id.clone())
                .collect::<Vec<_>>(),
        };
        plugin_ids.sort();
        plugin_ids.dedup();
        plugin_ids
    }

    fn ensure_plugin_running(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        let spawn_lock = {
            let registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin registry is unavailable".to_string(),
            })?;
            let record = registry
                .plugins
                .get(plugin_id)
                .ok_or_else(|| PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is not registered"),
                })?;
            record.spawn_lock.clone()
        };
        let _guard = spawn_lock.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("spawn lock for plugin '{plugin_id}' is poisoned"),
        })?;

        if let Some(process) = self.current_process(plugin_id) {
            self.record_activity(plugin_id);
            return Ok(process);
        }

        let bootstrap = self.prepare_spawn(plugin_id)?;
        let process = self.inner.factory.spawn(&bootstrap)?;
        let manager = self.clone();
        let plugin_id_for_exit = plugin_id.to_string();
        process.set_exit_listener(Arc::new(move |exit| {
            manager.handle_process_exit(&plugin_id_for_exit, exit);
        }));
        self.register_process(plugin_id, process.clone(), now_ms());

        if let Err(error) = process.wait_for_ready(self.activation_timeout()) {
            self.fail_plugin(
                plugin_id,
                &error.code,
                error.message,
                None,
                process.stderr_snapshot(),
            );
            let _ = process.kill();
            return Err(PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' failed before ready"),
            });
        }
        self.transition_plugin(plugin_id, PluginState::Loaded)?;

        if let Err(error) = process.activate(self.activation_timeout()) {
            self.fail_plugin(
                plugin_id,
                &error.code,
                error.message,
                None,
                process.stderr_snapshot(),
            );
            let _ = process.kill();
            return Err(PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' failed to activate"),
            });
        }
        self.transition_plugin(plugin_id, PluginState::Active)?;
        self.transition_plugin(plugin_id, PluginState::Running)?;
        self.record_activity(plugin_id);
        self.start_watchdog(plugin_id.to_string(), process.clone());

        Ok(process)
    }

    fn handle_process_exit(&self, plugin_id: &str, exit: ProcessExitInfo) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            match record.lifecycle.current_state() {
                PluginState::Deactivating | PluginState::Terminated | PluginState::Disabled => {
                    let _ = record.lifecycle.transition(PluginState::Terminated);
                }
                PluginState::Failed => {}
                _ => record.lifecycle.fail(
                    plugin_id,
                    PLUGIN_RUNTIME_ERROR_CODE,
                    format!(
                        "plugin process exited unexpectedly with code {:?}",
                        exit.code
                    ),
                    Some(serde_json::json!({ "exitCode": exit.code })),
                    None,
                ),
            }
        }
    }

    fn current_process(&self, plugin_id: &str) -> Option<Arc<dyn PluginProcessHandle>> {
        let Ok(registry) = self.inner.registry.lock() else {
            return None;
        };
        let record = registry.plugins.get(plugin_id)?;
        if matches!(
            record.lifecycle.current_state(),
            PluginState::Active | PluginState::Running
        ) {
            record.process.clone()
        } else {
            None
        }
    }

    fn prepare_spawn(&self, plugin_id: &str) -> Result<PluginBootstrapConfig, PluginRuntimeError> {
        let mut registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin registry is unavailable".to_string(),
        })?;
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not registered"),
            })?;

        match record.lifecycle.current_state() {
            PluginState::Validated | PluginState::Terminated => {
                record
                    .lifecycle
                    .transition(PluginState::Spawning)
                    .map_err(|message| PluginRuntimeError {
                        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                        message,
                    })?;
            }
            PluginState::Active | PluginState::Running => {}
            PluginState::Disabled => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is disabled"),
                });
            }
            PluginState::Failed => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                    message: format!("plugin '{plugin_id}' is in failed state"),
                });
            }
            other => {
                return Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: format!(
                        "plugin '{plugin_id}' cannot be spawned from state {:?}",
                        other
                    ),
                });
            }
        }

        let data_root = record.data_root.clone().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("plugin '{plugin_id}' is missing a data root"),
        })?;
        Ok(PluginBootstrapConfig {
            plugin_id: record.manifest.id.clone(),
            capabilities: record.effective_capabilities.iter().cloned().collect(),
            data_root: data_root.display().to_string(),
            delegated_grants: Vec::new(),
            host_ipc_settings: HostIpcSettings {
                heartbeat_interval_ms: self.inner.config.limits.heartbeat_interval_ms,
                heartbeat_timeout_ms: self.inner.config.limits.heartbeat_timeout_ms,
                call_timeout_ms: self.inner.config.limits.call_timeout_ms,
                max_inflight: 64,
                max_queue_depth: 256,
            },
        })
    }

    fn register_process(
        &self,
        plugin_id: &str,
        process: Arc<dyn PluginProcessHandle>,
        started_at_ms: u64,
    ) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.metrics.pid = process.process_id();
            record.metrics.started_at_ms = Some(started_at_ms);
            record.process = Some(process);
        }
    }

    fn transition_plugin(
        &self,
        plugin_id: &str,
        next_state: PluginState,
    ) -> Result<(), PluginRuntimeError> {
        let mut registry = self.inner.registry.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin registry is unavailable".to_string(),
        })?;
        let record = registry
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginRuntimeError {
                code: PLUGIN_NOT_AVAILABLE_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not registered"),
            })?;
        record
            .lifecycle
            .transition(next_state)
            .map_err(|message| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message,
            })
    }

    fn record_activity(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    fn mark_request_started(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.pending_requests += 1;
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    fn mark_request_finished(&self, plugin_id: &str) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.pending_requests = record.pending_requests.saturating_sub(1);
            record.metrics.last_activity_ms = Some(now_ms());
        }
    }

    fn fail_plugin(
        &self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) {
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            record
                .lifecycle
                .fail(plugin_id, code, message, details, stderr);
        }
    }

    fn deactivate_plugin(&self, plugin_id: &str) {
        let process = {
            let Ok(mut registry) = self.inner.registry.lock() else {
                return;
            };
            let Some(record) = registry.plugins.get_mut(plugin_id) else {
                return;
            };
            if !matches!(
                record.lifecycle.current_state(),
                PluginState::Active | PluginState::Running | PluginState::Failed
            ) {
                record.process = None;
                return;
            }
            if record.lifecycle.current_state() != PluginState::Failed {
                let _ = record.lifecycle.transition(PluginState::Deactivating);
            }
            record.process.clone()
        };

        let Some(process) = process else {
            return;
        };
        let result = process.deactivate(self.deactivation_timeout());
        if let Ok(mut registry) = self.inner.registry.lock()
            && let Some(record) = registry.plugins.get_mut(plugin_id)
        {
            record.process = None;
            record.pending_requests = 0;
            match result {
                Ok(()) => {
                    let _ = record.lifecycle.transition(PluginState::Terminated);
                }
                Err(error) => {
                    record.lifecycle.fail(
                        plugin_id,
                        &error.code,
                        error.message,
                        None,
                        process.stderr_snapshot(),
                    );
                }
            }
        }
    }

    fn activation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.activation_timeout_ms)
    }

    fn deactivation_timeout(&self) -> Duration {
        Duration::from_millis(self.inner.config.limits.deactivation_timeout_ms)
    }

    fn start_watchdog(&self, plugin_id: String, process: Arc<dyn PluginProcessHandle>) {
        let manager = self.clone();
        let idle_timeout = Duration::from_millis(self.inner.config.spawning.idle_timeout_ms);
        let heartbeat_interval =
            Duration::from_millis(self.inner.config.limits.heartbeat_interval_ms);
        let heartbeat_timeout =
            Duration::from_millis(self.inner.config.limits.heartbeat_timeout_ms);

        let _ = thread::Builder::new()
            .name(format!("volt-plugin-watchdog-{plugin_id}"))
            .spawn(move || {
                let mut missed = 0_u32;
                loop {
                    thread::sleep(heartbeat_interval);

                    let should_stop = {
                        let Ok(registry) = manager.inner.registry.lock() else {
                            return;
                        };
                        let Some(record) = registry.plugins.get(&plugin_id) else {
                            return;
                        };
                        !matches!(
                            record.lifecycle.current_state(),
                            PluginState::Active | PluginState::Running
                        ) || record
                            .process
                            .as_ref()
                            .map(|current| !Arc::ptr_eq(current, &process))
                            .unwrap_or(true)
                    };
                    if should_stop {
                        return;
                    }

                    if idle_timeout.as_millis() > 0 {
                        let should_idle_shutdown = {
                            let Ok(registry) = manager.inner.registry.lock() else {
                                return;
                            };
                            let Some(record) = registry.plugins.get(&plugin_id) else {
                                return;
                            };
                            record.pending_requests == 0
                                && record
                                    .metrics
                                    .last_activity_ms
                                    .map(|last_activity_ms| {
                                        now_ms().saturating_sub(last_activity_ms)
                                            >= idle_timeout.as_millis() as u64
                                    })
                                    .unwrap_or(false)
                        };
                        if should_idle_shutdown {
                            manager.deactivate_plugin(&plugin_id);
                            return;
                        }
                    }

                    {
                        if let Ok(mut registry) = manager.inner.registry.lock()
                            && let Some(record) = registry.plugins.get_mut(&plugin_id)
                        {
                            record.metrics.last_heartbeat_sent_ms = Some(now_ms());
                        }
                    }

                    match process.heartbeat(heartbeat_timeout) {
                        Ok(()) => {
                            missed = 0;
                            if let Ok(mut registry) = manager.inner.registry.lock()
                                && let Some(record) = registry.plugins.get_mut(&plugin_id)
                            {
                                record.metrics.last_heartbeat_ack_ms = Some(now_ms());
                                record.metrics.missed_heartbeats = 0;
                            }
                        }
                        Err(error) => {
                            missed += 1;
                            if let Ok(mut registry) = manager.inner.registry.lock()
                                && let Some(record) = registry.plugins.get_mut(&plugin_id)
                            {
                                record.metrics.heartbeat_failures += 1;
                                record.metrics.missed_heartbeats = missed;
                            }
                            if missed >= 2 {
                                manager.fail_plugin(
                                    &plugin_id,
                                    PLUGIN_HEARTBEAT_TIMEOUT_CODE,
                                    error.message,
                                    None,
                                    process.stderr_snapshot(),
                                );
                                let _ = process.kill();
                                return;
                            }
                        }
                    }
                }
            });
    }

    fn discover_plugin_record(
        &self,
        manifest_path: &Path,
        enabled_plugins: &HashSet<String>,
    ) -> Result<PluginRecord, PluginDiscoveryIssue> {
        let plugin_root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| PluginDiscoveryIssue {
                path: Some(manifest_path.to_path_buf()),
                message: "manifest file is missing a parent directory".to_string(),
            })?;
        let manifest_bytes = fs::read(manifest_path).map_err(|error| PluginDiscoveryIssue {
            path: Some(manifest_path.to_path_buf()),
            message: format!("failed to read manifest: {error}"),
        })?;
        let manifest = parse_plugin_manifest(&manifest_bytes, &plugin_root).map_err(|message| {
            PluginDiscoveryIssue {
                path: Some(manifest_path.to_path_buf()),
                message,
            }
        })?;
        let enabled = enabled_plugins.contains(&manifest.id);
        let requested_capabilities = manifest
            .capabilities
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let effective_capabilities = compute_effective_capabilities(
            &manifest,
            &self.inner.config,
            &self.inner.app_permissions,
        );
        let data_root = if enabled {
            Some(ensure_plugin_data_root(
                &self.inner.app_data_root,
                &manifest.id,
            )?)
        } else {
            None
        };

        let mut lifecycle = PluginLifecycle::new();
        lifecycle
            .transition(PluginState::Discovered)
            .expect("discover");
        if !enabled {
            lifecycle
                .transition(PluginState::Disabled)
                .expect("disable");
        } else if requested_capabilities != effective_capabilities {
            let missing = requested_capabilities
                .difference(&effective_capabilities)
                .cloned()
                .collect::<Vec<_>>();
            lifecycle.fail(
                &manifest.id,
                PLUGIN_NOT_AVAILABLE_CODE,
                format!(
                    "requested capabilities are unsatisfiable: {}",
                    missing.join(", ")
                ),
                Some(serde_json::json!({ "missingCapabilities": missing })),
                None,
            );
        } else {
            lifecycle
                .transition(PluginState::Validated)
                .expect("validate");
        }

        Ok(PluginRecord {
            manifest,
            manifest_path: manifest_path.to_path_buf(),
            enabled,
            data_root,
            #[cfg(test)]
            requested_capabilities,
            effective_capabilities,
            lifecycle,
            metrics: PluginResourceMetrics::default(),
            process: None,
            pending_requests: 0,
            spawn_lock: Arc::new(Mutex::new(())),
        })
    }
}

struct ChildPluginProcess {
    inner: Arc<ChildPluginProcessInner>,
}

struct ChildPluginProcessInner {
    child: Mutex<Child>,
    stdin: Mutex<BufWriter<ChildStdin>>,
    waiters: Mutex<HashMap<String, mpsc::Sender<WireMessage>>>,
    ready: ReadyState,
    exit: ExitState,
    next_id: AtomicU64,
    stderr: Arc<Mutex<String>>,
}

struct ReadyState {
    ready: Mutex<bool>,
    condvar: Condvar,
}

struct ExitState {
    info: Mutex<Option<ProcessExitInfo>>,
    condvar: Condvar,
    listener: Mutex<Option<ExitListener>>,
}

impl Default for ReadyState {
    fn default() -> Self {
        Self {
            ready: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }
}

impl Default for ExitState {
    fn default() -> Self {
        Self {
            info: Mutex::new(None),
            condvar: Condvar::new(),
            listener: Mutex::new(None),
        }
    }
}

impl PluginProcessFactory for RealPluginProcessFactory {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        let binary = resolve_plugin_host_binary()?;
        let config_json = serde_json::to_vec(config).map_err(|error| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("failed to serialize plugin bootstrap config: {error}"),
        })?;
        let config_b64 = BASE64.encode(config_json);
        let mut child = Command::new(binary)
            .arg("--plugin")
            .arg("--config")
            .arg(config_b64)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: format!("failed to spawn plugin host: {error}"),
            })?;
        let stdin = child.stdin.take().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin host stdin was not captured".to_string(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin host stdout was not captured".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin host stderr was not captured".to_string(),
        })?;
        Ok(Arc::new(ChildPluginProcess::new(
            child, stdin, stdout, stderr,
        )))
    }
}

impl ChildPluginProcess {
    fn new(
        child: Child,
        stdin: ChildStdin,
        stdout: ChildStdout,
        mut stderr: impl Read + Send + 'static,
    ) -> Self {
        let stderr_buffer = Arc::new(Mutex::new(String::new()));
        let inner = Arc::new(ChildPluginProcessInner {
            child: Mutex::new(child),
            stdin: Mutex::new(BufWriter::new(stdin)),
            waiters: Mutex::new(HashMap::new()),
            ready: ReadyState::default(),
            exit: ExitState::default(),
            next_id: AtomicU64::new(1),
            stderr: stderr_buffer.clone(),
        });

        {
            let process = inner.clone();
            let _ = thread::Builder::new()
                .name("volt-plugin-host-stdout".to_string())
                .spawn(move || read_plugin_stdout(process, stdout));
        }
        {
            let process = inner.clone();
            let _ = thread::Builder::new()
                .name("volt-plugin-host-exit".to_string())
                .spawn(move || {
                    let exit_code = process
                        .child
                        .lock()
                        .ok()
                        .and_then(|mut child| child.wait().ok())
                        .and_then(|status| status.code());
                    notify_exit(&process.exit, ProcessExitInfo { code: exit_code });
                });
        }
        {
            let _ = thread::Builder::new()
                .name("volt-plugin-host-stderr".to_string())
                .spawn(move || {
                    let mut captured = String::new();
                    let _ = stderr.read_to_string(&mut captured);
                    if let Ok(mut buffer) = stderr_buffer.lock() {
                        *buffer = captured;
                    }
                });
        }

        Self { inner }
    }

    fn next_id(&self) -> String {
        format!(
            "plugin-{}",
            self.inner.next_id.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn send_and_wait(
        &self,
        message: WireMessage,
        timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError> {
        let (tx, rx) = mpsc::channel();
        self.inner
            .waiters
            .lock()
            .map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin waiter map is unavailable".to_string(),
            })?
            .insert(message.id.clone(), tx);
        if let Err(error) = write_wire_message(
            &mut *self.inner.stdin.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin stdin is unavailable".to_string(),
            })?,
            &message,
        ) {
            let _ = self
                .inner
                .waiters
                .lock()
                .map(|mut waiters| waiters.remove(&message.id));
            return Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: format!("failed to write message to plugin host: {error}"),
            });
        }

        rx.recv_timeout(timeout).map_err(|error| {
            let _ = self
                .inner
                .waiters
                .lock()
                .map(|mut waiters| waiters.remove(&message.id));
            let code = if matches!(error, mpsc::RecvTimeoutError::Timeout) {
                "TIMEOUT"
            } else {
                PLUGIN_RUNTIME_ERROR_CODE
            };
            PluginRuntimeError {
                code: code.to_string(),
                message: format!("plugin did not respond in {}ms", timeout.as_millis()),
            }
        })
    }
}

impl PluginProcessHandle for ChildPluginProcess {
    fn process_id(&self) -> Option<u32> {
        self.inner.child.lock().ok().map(|child| child.id())
    }

    fn wait_for_ready(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let mut ready = self
            .inner
            .ready
            .ready
            .lock()
            .map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin ready state is unavailable".to_string(),
            })?;
        if *ready {
            return Ok(());
        }

        let deadline = Instant::now() + timeout;
        loop {
            if self
                .inner
                .exit
                .info
                .lock()
                .ok()
                .and_then(|info| info.clone())
                .is_some()
            {
                return Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "plugin exited before sending ready".to_string(),
                });
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(PluginRuntimeError {
                    code: "TIMEOUT".to_string(),
                    message: format!("plugin did not send ready within {}ms", timeout.as_millis()),
                });
            }
            let (next_ready, _) = self
                .inner
                .ready
                .condvar
                .wait_timeout(ready, deadline.saturating_duration_since(now))
                .map_err(|_| PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "plugin ready wait failed".to_string(),
                })?;
            ready = next_ready;
            if *ready {
                return Ok(());
            }
        }
    }

    fn activate(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = self.send_and_wait(
            WireMessage::signal(self.next_id(), "activate", None),
            timeout,
        )?;
        if let Some(error) = response.error {
            return Err(PluginRuntimeError {
                code: error.code,
                message: error.message,
            });
        }
        Ok(())
    }

    fn request(
        &self,
        method: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError> {
        self.send_and_wait(
            WireMessage::request(self.next_id(), method.to_string(), payload),
            timeout,
        )
    }

    fn heartbeat(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = self.send_and_wait(
            WireMessage::signal(self.next_id(), "heartbeat", None),
            timeout,
        )?;
        if response.message_type == WireMessageType::Signal && response.method == "heartbeat-ack" {
            Ok(())
        } else {
            Err(PluginRuntimeError {
                code: PLUGIN_HEARTBEAT_TIMEOUT_CODE.to_string(),
                message: "plugin heartbeat ack was invalid".to_string(),
            })
        }
    }

    fn deactivate(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        write_wire_message(
            &mut *self.inner.stdin.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin stdin is unavailable".to_string(),
            })?,
            &WireMessage::signal(self.next_id(), "deactivate", None),
        )
        .map_err(|error| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("failed to send deactivate signal: {error}"),
        })?;
        if wait_for_exit(&self.inner.exit, timeout).is_some() {
            return Ok(());
        }
        let _ = self.kill();
        if wait_for_exit(
            &self.inner.exit,
            Duration::from_millis(DEFAULT_EXIT_WAIT_AFTER_KILL_MS),
        )
        .is_some()
        {
            return Ok(());
        }
        Err(PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin did not exit after deactivation timeout".to_string(),
        })
    }

    fn kill(&self) -> Result<(), PluginRuntimeError> {
        self.inner
            .child
            .lock()
            .map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin child process is unavailable".to_string(),
            })?
            .kill()
            .map_err(|error| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: format!("failed to kill plugin process: {error}"),
            })
    }

    fn set_exit_listener(&self, listener: Arc<dyn Fn(ProcessExitInfo) + Send + Sync>) {
        if let Some(exit) = self
            .inner
            .exit
            .info
            .lock()
            .ok()
            .and_then(|info| info.clone())
        {
            listener(exit);
            return;
        }
        if let Ok(mut current) = self.inner.exit.listener.lock() {
            *current = Some(listener);
        }
    }

    fn stderr_snapshot(&self) -> Option<String> {
        self.inner
            .stderr
            .lock()
            .ok()
            .and_then(|stderr| (!stderr.is_empty()).then(|| stderr.clone()))
    }
}

fn resolve_plugin_host_binary() -> Result<PathBuf, PluginRuntimeError> {
    if let Ok(path) = std::env::var(PLUGIN_HOST_PATH_ENV) {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let current_exe = std::env::current_exe().map_err(|error| PluginRuntimeError {
        code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
        message: format!("failed to resolve current executable: {error}"),
    })?;
    let binary_name = if cfg!(windows) {
        "volt-plugin-host.exe"
    } else {
        "volt-plugin-host"
    };

    let mut candidates = vec![current_exe.with_file_name(binary_name)];
    if let Some(parent) = current_exe.parent() {
        candidates.push(parent.join(binary_name));
        if parent.file_name().and_then(|value| value.to_str()) == Some("deps")
            && let Some(grand_parent) = parent.parent()
        {
            candidates.push(grand_parent.join(binary_name));
        }
    }

    candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "failed to locate volt-plugin-host binary".to_string(),
        })
}

fn read_plugin_stdout(process: Arc<ChildPluginProcessInner>, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout);
    loop {
        let message = match read_wire_message(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => return,
            Err(_) => return,
        };

        if message.message_type == WireMessageType::Signal && message.method == "ready" {
            if let Ok(mut ready) = process.ready.ready.lock() {
                *ready = true;
                process.ready.condvar.notify_all();
            }
            continue;
        }

        if let Ok(mut waiters) = process.waiters.lock()
            && let Some(waiter) = waiters.remove(&message.id)
        {
            let _ = waiter.send(message);
        }
    }
}

fn notify_exit(exit_state: &ExitState, exit: ProcessExitInfo) {
    if let Ok(mut info) = exit_state.info.lock()
        && info.is_none()
    {
        *info = Some(exit.clone());
    }
    exit_state.condvar.notify_all();
    if let Ok(listener) = exit_state.listener.lock()
        && let Some(listener) = listener.clone()
    {
        listener(exit);
    }
}

fn wait_for_exit(exit_state: &ExitState, timeout: Duration) -> Option<ProcessExitInfo> {
    let mut info = exit_state.info.lock().ok()?;
    if info.is_some() {
        return info.clone();
    }
    let (next_info, _) = exit_state.condvar.wait_timeout(info, timeout).ok()?;
    info = next_info;
    info.clone()
}

fn read_wire_message<R: Read>(reader: &mut BufReader<R>) -> std::io::Result<Option<WireMessage>> {
    let mut len_buf = [0_u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }

    let length = u32::from_le_bytes(len_buf) as usize;
    if length == 0 || length > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid frame length: {length}"),
        ));
    }

    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body)?;
    let raw = String::from_utf8(body)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let trimmed = raw.trim_end_matches('\n');
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn write_wire_message<W: Write>(writer: &mut W, message: &WireMessage) -> std::io::Result<()> {
    let json = serde_json::to_string(message)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let body = format!("{json}\n");
    let bytes = body.as_bytes();
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("frame too large: {}", bytes.len()),
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()
}

#[cfg(test)]
impl PluginRecord {
    fn snapshot(&self) -> PluginSnapshot {
        PluginSnapshot {
            plugin_id: self.manifest.id.clone(),
            state: self.lifecycle.current_state(),
            enabled: self.enabled,
            manifest_path: self.manifest_path.clone(),
            data_root: self.data_root.clone(),
            requested_capabilities: self.requested_capabilities.iter().cloned().collect(),
            effective_capabilities: self.effective_capabilities.iter().cloned().collect(),
            transitions: self.lifecycle.transitions.clone(),
            errors: self.lifecycle.errors.clone(),
            metrics: self.metrics.clone(),
            process_running: self.process.is_some(),
        }
    }
}

impl PluginLifecycle {
    fn new() -> Self {
        Self {
            state: PluginState::Discovered,
            transitions: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn transition(&mut self, next_state: PluginState) -> Result<(), String> {
        if !is_valid_transition(self.current_state(), next_state) {
            return Err(format!(
                "invalid plugin state transition: {:?} -> {:?}",
                self.current_state(),
                next_state
            ));
        }
        let previous_state = self
            .transitions
            .last()
            .map(|transition| transition.new_state);
        self.state = next_state;
        self.transitions.push(PluginStateTransition {
            previous_state,
            new_state: next_state,
            timestamp_ms: now_ms(),
        });
        Ok(())
    }

    fn fail(
        &mut self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) {
        let _ = self.transition(PluginState::Failed);
        self.errors.push(PluginError {
            plugin_id: plugin_id.to_string(),
            state: PluginState::Failed,
            code: code.to_string(),
            message,
            details,
            stderr,
            timestamp_ms: now_ms(),
        });
    }

    fn current_state(&self) -> PluginState {
        self.transitions
            .last()
            .map(|transition| transition.new_state)
            .unwrap_or(self.state)
    }
}

fn parse_plugin_manifest(contents: &[u8], plugin_root: &Path) -> Result<PluginManifest, String> {
    let value: Value = serde_json::from_slice(contents)
        .map_err(|error| format!("manifest is not valid JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "manifest must be a JSON object".to_string())?;

    let id = required_string_field(object, "id")?;
    if !is_valid_reverse_domain(&id) {
        return Err("manifest id must be in reverse-domain format".to_string());
    }
    let _name = required_string_field(object, "name")?;
    let version = required_string_field(object, "version")?;
    Version::parse(&version)
        .map_err(|error| format!("manifest version must be valid semver: {error}"))?;
    let api_version = object
        .get("apiVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest apiVersion must be a positive integer".to_string())?;
    if !SUPPORTED_PLUGIN_API_VERSIONS.contains(&api_version) {
        return Err(format!("unsupported plugin apiVersion '{api_version}'"));
    }

    let engine = object
        .get("engine")
        .and_then(Value::as_object)
        .ok_or_else(|| "manifest engine must be an object".to_string())?;
    let engine_volt = required_string_field(engine, "volt")?;
    let version_req = VersionReq::parse(&engine_volt)
        .map_err(|error| format!("manifest engine.volt must be a valid semver range: {error}"))?;
    let host_version = Version::parse(HOST_VOLT_VERSION)
        .map_err(|error| format!("failed to parse host version: {error}"))?;
    if !version_req.matches(&host_version) {
        return Err(format!(
            "plugin requires Volt '{engine_volt}', host version is '{HOST_VOLT_VERSION}'"
        ));
    }

    let backend = required_string_field(object, "backend")?;
    if !backend.ends_with(".js") && !backend.ends_with(".mjs") {
        return Err("manifest backend must end with .js or .mjs".to_string());
    }
    let backend_path = plugin_root.join(&backend);
    if !backend_path.is_file() {
        return Err(format!(
            "plugin backend entry '{}' does not exist",
            backend_path.display()
        ));
    }

    let capabilities = object
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest capabilities must be an array".to_string())?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let capability = value
                .as_str()
                .ok_or_else(|| format!("manifest capabilities[{index}] must be a string"))?;
            if Permission::from_str_name(capability).is_none() {
                return Err(format!(
                    "manifest capabilities[{index}] contains unknown capability '{capability}'"
                ));
            }
            Ok(capability.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = HashSet::new();
    for capability in &capabilities {
        if !seen.insert(capability.clone()) {
            return Err(format!(
                "manifest capabilities contains duplicate capability '{capability}'"
            ));
        }
    }

    if let Some(contributes) = object.get("contributes") {
        validate_contributes(contributes)?;
    }
    if let Some(signature) = object.get("signature") {
        validate_signature(signature)?;
    }

    Ok(PluginManifest { id, capabilities })
}

fn validate_contributes(value: &Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err("manifest contributes must be an object".to_string());
    };
    if let Some(commands) = object.get("commands") {
        let Some(commands) = commands.as_array() else {
            return Err("manifest contributes.commands must be an array".to_string());
        };
        for (index, command) in commands.iter().enumerate() {
            let Some(command) = command.as_object() else {
                return Err(format!(
                    "manifest contributes.commands[{index}] must be an object"
                ));
            };
            required_string_field(command, "id").map_err(|_| {
                format!("manifest contributes.commands[{index}].id must be a non-empty string")
            })?;
            required_string_field(command, "title").map_err(|_| {
                format!("manifest contributes.commands[{index}].title must be a non-empty string")
            })?;
        }
    }
    Ok(())
}

fn validate_signature(value: &Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err("manifest signature must be an object".to_string());
    };
    required_string_field(object, "algorithm")
        .map_err(|_| "manifest signature.algorithm must be a non-empty string".to_string())?;
    required_string_field(object, "value")
        .map_err(|_| "manifest signature.value must be a non-empty string".to_string())?;
    Ok(())
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("manifest {field} must be a non-empty string"))?;
    Ok(value.to_string())
}

fn compute_effective_capabilities(
    manifest: &PluginManifest,
    config: &RunnerPluginConfig,
    app_permissions: &HashSet<Permission>,
) -> BTreeSet<String> {
    let requested = manifest
        .capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let host_grants = config
        .grants
        .get(&manifest.id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let app_permissions = app_permissions
        .iter()
        .map(|permission| permission.as_str().to_string())
        .collect::<BTreeSet<_>>();
    requested
        .intersection(&host_grants)
        .cloned()
        .collect::<BTreeSet<_>>()
        .intersection(&app_permissions)
        .cloned()
        .collect()
}

fn parse_plugin_route(method: &str) -> Result<Option<PluginRoute>, String> {
    if !method.starts_with("plugin:") {
        return Ok(None);
    }
    let route = method.trim_start_matches("plugin:");
    let Some((plugin_id, channel)) = route.split_once(':') else {
        return Err("plugin IPC routes must use 'plugin:<plugin-id>:<channel>'".to_string());
    };
    if plugin_id.trim().is_empty() || channel.trim().is_empty() {
        return Err("plugin IPC routes must include both plugin id and channel".to_string());
    }
    Ok(Some(PluginRoute {
        plugin_id: plugin_id.to_string(),
        method: channel.to_string(),
    }))
}

fn resolve_plugin_directory(path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(candidate)
    }
}

fn collect_manifest_paths(
    directory: &Path,
    manifest_paths: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_manifest_paths(&path, manifest_paths)?;
        } else if path.file_name().and_then(|value| value.to_str()) == Some(MANIFEST_FILE_NAME) {
            manifest_paths.push(path);
        }
    }
    Ok(())
}

fn resolve_app_data_root(app_name: &str) -> Result<PathBuf, String> {
    let base = dirs::data_local_dir()
        .or_else(|| std::env::current_dir().ok())
        .ok_or_else(|| "failed to resolve app data directory".to_string())?;
    let root = base.join("volt").join(sanitize_app_namespace(app_name));
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create app data directory '{}': {error}",
            root.display()
        )
    })?;
    Ok(root)
}

fn ensure_plugin_data_root(
    app_data_root: &Path,
    plugin_id: &str,
) -> Result<PathBuf, PluginDiscoveryIssue> {
    let data_root = app_data_root.join("plugins").join(plugin_id);
    fs::create_dir_all(&data_root).map_err(|error| PluginDiscoveryIssue {
        path: Some(data_root.clone()),
        message: format!("failed to create plugin data root: {error}"),
    })?;
    Ok(data_root)
}

fn sanitize_app_namespace(app_name: &str) -> String {
    let sanitized = app_name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let compact = sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        "app".to_string()
    } else {
        compact
    }
}

fn is_valid_reverse_domain(id: &str) -> bool {
    let segments = id.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        return false;
    }
    segments.iter().all(|segment| {
        let mut chars = segment.chars();
        match chars.next() {
            Some(first) if first.is_ascii_lowercase() => {
                chars.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
            }
            _ => false,
        }
    })
}

fn is_valid_transition(current: PluginState, next: PluginState) -> bool {
    if current == next || next == PluginState::Failed || next == PluginState::Disabled {
        return true;
    }
    matches!(
        (current, next),
        (PluginState::Discovered, PluginState::Validated)
            | (PluginState::Validated, PluginState::Spawning)
            | (PluginState::Terminated, PluginState::Spawning)
            | (PluginState::Spawning, PluginState::Loaded)
            | (PluginState::Loaded, PluginState::Active)
            | (PluginState::Active, PluginState::Running)
            | (PluginState::Active, PluginState::Deactivating)
            | (PluginState::Running, PluginState::Deactivating)
            | (PluginState::Deactivating, PluginState::Terminated)
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}

#[cfg(test)]
mod tests;
