use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
#[cfg(test)]
use volt_core::ipc::IPC_HANDLER_TIMEOUT_CODE;
use volt_core::permissions::Permission;

use crate::runner::config::RunnerPluginConfig;

mod access;
mod contracts;
mod discovery;
mod host_api;
mod host_api_access;
mod host_api_fs;
mod host_api_helpers;
mod host_api_storage;
mod host_api_support;
mod lifecycle;
mod lifecycle_bus;
mod manifest;
mod paths;
mod process;
mod runtime;
mod watchdog;

use self::access::{NativePluginAccessPicker, PluginAccessPicker};
use self::lifecycle::{PluginLifecycle, now_ms};
use self::lifecycle_bus::LifecycleBus;
use self::manifest::{compute_effective_capabilities, parse_plugin_manifest, parse_plugin_route};
use self::paths::{
    collect_manifest_paths, ensure_plugin_data_root, resolve_app_data_root,
    resolve_plugin_directory,
};
use self::process::RealPluginProcessFactory;
#[cfg(test)]
use self::process::{WireError, WireMessageType};
pub(crate) use self::{
    access::AccessDialogRequest,
    contracts::{
        DelegatedGrant, ExitListener, HostIpcSettings, MessageListener, PluginBootstrapConfig,
        PluginProcessFactory, PluginProcessHandle, PluginRuntimeError, ProcessExitInfo,
    },
};

const PLUGIN_ACCESS_ERROR_CODE: &str = "PLUGIN_ACCESS_ERROR";
const PLUGIN_COMMAND_NOT_FOUND_CODE: &str = "PLUGIN_COMMAND_NOT_FOUND";
const PLUGIN_FS_ERROR_CODE: &str = "PLUGIN_FS_ERROR";
const PLUGIN_HEARTBEAT_TIMEOUT_CODE: &str = "PLUGIN_HEARTBEAT_TIMEOUT";
const PLUGIN_IPC_HANDLER_NOT_FOUND_CODE: &str = "PLUGIN_IPC_HANDLER_NOT_FOUND";
const PLUGIN_NOT_AVAILABLE_CODE: &str = "PLUGIN_NOT_AVAILABLE";
const PLUGIN_ROUTE_INVALID_CODE: &str = "PLUGIN_ROUTE_INVALID";
const PLUGIN_RUNTIME_ERROR_CODE: &str = "PLUGIN_RUNTIME_ERROR";
const PLUGIN_STORAGE_ERROR_CODE: &str = "PLUGIN_STORAGE_ERROR";
const PLUGIN_AUTO_DISABLED_CODE: &str = "PLUGIN_AUTO_DISABLED";
const DEFAULT_PRE_SPAWN_GRACE_MS: u64 = 50;
const DEFAULT_PLUGIN_ERROR_HISTORY_LIMIT: usize = 50;

pub(crate) use self::lifecycle_bus::{PluginLifecycleEvent, SubscriptionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginStateTransition {
    pub(crate) previous_state: Option<PluginState>,
    pub(crate) new_state: PluginState,
    pub(crate) timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginError {
    pub(crate) plugin_id: String,
    pub(crate) state: PluginState,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
    pub(crate) stderr: Option<String>,
    pub(crate) timestamp_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginResourceMetrics {
    pub(crate) pid: Option<u32>,
    pub(crate) started_at_ms: Option<u64>,
    pub(crate) last_activity_ms: Option<u64>,
    pub(crate) last_heartbeat_sent_ms: Option<u64>,
    pub(crate) last_heartbeat_ack_ms: Option<u64>,
    pub(crate) missed_heartbeats: u32,
    pub(crate) heartbeat_failures: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginDiscoveryIssue {
    pub(crate) path: Option<PathBuf>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginRegistrationSnapshot {
    pub(crate) command_count: usize,
    pub(crate) event_subscription_count: usize,
    pub(crate) ipc_handler_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginStateSnapshot {
    pub(crate) plugin_id: String,
    pub(crate) current_state: PluginState,
    pub(crate) enabled: bool,
    pub(crate) manifest_path: PathBuf,
    pub(crate) data_root: Option<PathBuf>,
    pub(crate) requested_capabilities: Vec<String>,
    pub(crate) effective_capabilities: Vec<String>,
    pub(crate) transition_history: Vec<PluginStateTransition>,
    pub(crate) errors: Vec<PluginError>,
    pub(crate) metrics: PluginResourceMetrics,
    pub(crate) process_running: bool,
    pub(crate) active_registrations: PluginRegistrationSnapshot,
    pub(crate) delegated_grant_count: usize,
    pub(crate) consecutive_failures: u32,
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
    access_picker: Arc<dyn PluginAccessPicker>,
    error_history_limit: usize,
    lifecycle_bus: LifecycleBus,
    registry: Mutex<PluginRegistry>,
}

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
    delegated_grants: HashSet<String>,
    storage_reconciled: bool,
    spawn_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Clone)]
struct PluginManifest {
    id: String,
    name: String,
    capabilities: Vec<String>,
    backend_entry: PathBuf,
    #[allow(dead_code)]
    prefetch_on: Vec<String>,
    raw_manifest: Value,
}

#[derive(Debug, Clone)]
struct PluginRoute {
    plugin_id: String,
    method: String,
}

#[derive(Debug, Clone)]
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

impl PluginRegistry {
    fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            discovery_issues: Vec::new(),
            commands: HashMap::new(),
            ipc_handlers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests;
