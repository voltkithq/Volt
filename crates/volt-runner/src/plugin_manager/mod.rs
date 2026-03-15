use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;
#[cfg(test)]
use volt_core::ipc::IPC_HANDLER_TIMEOUT_CODE;
use volt_core::permissions::Permission;

use crate::runner::config::RunnerPluginConfig;

mod discovery;
mod host_api;
mod host_api_helpers;
mod host_api_support;
mod lifecycle;
mod manifest;
mod paths;
mod process;
mod runtime;
mod watchdog;

use self::lifecycle::{PluginLifecycle, now_ms};
use self::manifest::{compute_effective_capabilities, parse_plugin_manifest, parse_plugin_route};
use self::paths::{
    collect_manifest_paths, ensure_plugin_data_root, resolve_app_data_root,
    resolve_plugin_directory,
};
use self::process::{RealPluginProcessFactory, WireMessage};
#[cfg(test)]
use self::process::{WireError, WireMessageType};

const PLUGIN_RUNTIME_ERROR_CODE: &str = "PLUGIN_RUNTIME_ERROR";
const PLUGIN_HEARTBEAT_TIMEOUT_CODE: &str = "PLUGIN_HEARTBEAT_TIMEOUT";
const PLUGIN_NOT_AVAILABLE_CODE: &str = "PLUGIN_NOT_AVAILABLE";
const PLUGIN_ROUTE_INVALID_CODE: &str = "PLUGIN_ROUTE_INVALID";
#[cfg(test)]
const PLUGIN_COMMAND_NOT_FOUND_CODE: &str = "PLUGIN_COMMAND_NOT_FOUND";
const PLUGIN_FS_ERROR_CODE: &str = "PLUGIN_FS_ERROR";
const PLUGIN_IPC_HANDLER_NOT_FOUND_CODE: &str = "PLUGIN_IPC_HANDLER_NOT_FOUND";
const DEFAULT_PRE_SPAWN_GRACE_MS: u64 = 50;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub(crate) struct PluginDiscoveryIssue {
    pub(crate) path: Option<PathBuf>,
    pub(crate) message: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    commands: HashMap<String, PluginCommandRoute>,
    ipc_handlers: HashMap<String, PluginRoute>,
}

struct PluginRecord {
    manifest: PluginManifest,
    manifest_path: PathBuf,
    enabled: bool,
    data_root: Option<PathBuf>,
    requested_capabilities: BTreeSet<String>,
    effective_capabilities: BTreeSet<String>,
    lifecycle: PluginLifecycle,
    metrics: PluginResourceMetrics,
    process: Option<Arc<dyn PluginProcessHandle>>,
    pending_requests: usize,
    registrations: PluginRegistrations,
    spawn_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
struct PluginManifest {
    id: String,
    capabilities: Vec<String>,
    backend_entry: PathBuf,
    raw_manifest: Value,
}

#[derive(Debug, Clone)]
struct PluginRoute {
    plugin_id: String,
    method: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PluginCommandRoute {
    plugin_id: String,
    command_id: String,
}

#[derive(Debug, Clone, Default)]
struct PluginRegistrations {
    commands: HashSet<String>,
    event_subscriptions: HashSet<String>,
    ipc_handlers: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PluginRuntimeError {
    code: String,
    message: String,
}

impl fmt::Display for PluginRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PluginRuntimeError {}

trait PluginProcessFactory: Send + Sync {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError>;
}

trait PluginProcessHandle: Send + Sync {
    fn process_id(&self) -> Option<u32>;
    fn wait_for_ready(&self, timeout: std::time::Duration) -> Result<(), PluginRuntimeError>;
    fn activate(&self, timeout: std::time::Duration) -> Result<(), PluginRuntimeError>;
    fn send_event(&self, method: &str, payload: Value) -> Result<(), PluginRuntimeError>;
    fn request(
        &self,
        method: &str,
        payload: Value,
        timeout: std::time::Duration,
    ) -> Result<WireMessage, PluginRuntimeError>;
    fn heartbeat(&self, timeout: std::time::Duration) -> Result<(), PluginRuntimeError>;
    fn deactivate(&self, timeout: std::time::Duration) -> Result<(), PluginRuntimeError>;
    fn kill(&self) -> Result<(), PluginRuntimeError>;
    fn set_exit_listener(&self, listener: Arc<dyn Fn(ProcessExitInfo) + Send + Sync>);
    fn set_message_listener(&self, listener: MessageListener);
    fn stderr_snapshot(&self) -> Option<String>;
}

#[derive(Debug, Clone)]
struct ProcessExitInfo {
    code: Option<i32>,
}

type ExitListener = Arc<dyn Fn(ProcessExitInfo) + Send + Sync>;
type MessageListener = Arc<dyn Fn(WireMessage) -> Option<WireMessage> + Send + Sync>;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginBootstrapConfig {
    plugin_id: String,
    backend_entry: String,
    manifest: Value,
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

#[cfg(test)]
mod tests;
