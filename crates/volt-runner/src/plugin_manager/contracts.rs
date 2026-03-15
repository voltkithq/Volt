use std::fmt;
use std::sync::Arc;

use serde_json::Value;

use super::process::WireMessage;

#[derive(Debug, Clone)]
pub(crate) struct PluginRuntimeError {
    pub(crate) code: String,
    pub(crate) message: String,
}

impl fmt::Display for PluginRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PluginRuntimeError {}

pub(crate) trait PluginProcessFactory: Send + Sync {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError>;
}

pub(crate) trait PluginProcessHandle: Send + Sync {
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
pub(crate) struct ProcessExitInfo {
    pub(crate) code: Option<i32>,
}

pub(crate) type ExitListener = Arc<dyn Fn(ProcessExitInfo) + Send + Sync>;
pub(crate) type MessageListener = Arc<dyn Fn(WireMessage) -> Option<WireMessage> + Send + Sync>;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginBootstrapConfig {
    pub(crate) plugin_id: String,
    pub(crate) backend_entry: String,
    pub(crate) manifest: Value,
    pub(crate) capabilities: Vec<String>,
    pub(crate) data_root: String,
    pub(crate) delegated_grants: Vec<DelegatedGrant>,
    pub(crate) host_ipc_settings: HostIpcSettings,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DelegatedGrant {
    pub(crate) grant_id: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HostIpcSettings {
    pub(crate) heartbeat_interval_ms: u64,
    pub(crate) heartbeat_timeout_ms: u64,
    pub(crate) call_timeout_ms: u64,
    pub(crate) max_inflight: u32,
    pub(crate) max_queue_depth: u32,
}
