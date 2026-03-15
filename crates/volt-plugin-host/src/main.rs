//! `volt-plugin-host` — lightweight Boa runner for plugin processes.
//!
//! This binary is spawned by the Plugin Host Manager in the app process.
//! It accepts `--plugin --config <base64-json>` CLI args, initializes a
//! Boa context with a scoped CapabilityGuard, and runs the IPC message loop.
//!
//! No wry, no tao, no WebView — Boa engine + serde + IPC only.

mod ipc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use std::io::{self, BufReader};
use volt_permissions::CapabilityGuard;

/// Host IPC settings received at spawn time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostIpcSettings {
    pub heartbeat_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub call_timeout_ms: u64,
    pub max_inflight: u32,
    pub max_queue_depth: u32,
}

impl Default for HostIpcSettings {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 5000,
            heartbeat_timeout_ms: 3000,
            call_timeout_ms: 30000,
            max_inflight: 64,
            max_queue_depth: 256,
        }
    }
}

/// A grant delegation entry received from the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedGrant {
    pub grant_id: String,
    pub path: String,
}

/// Configuration received from the host process via `--config <base64-json>`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
    pub data_root: String,
    #[serde(default)]
    pub delegated_grants: Vec<DelegatedGrant>,
    #[serde(default)]
    pub host_ipc_settings: Option<HostIpcSettings>,
}

fn init_logging() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}

fn parse_args() -> Result<PluginConfig, String> {
    let args: Vec<String> = std::env::args().collect();

    let has_plugin_flag = args.iter().any(|a| a == "--plugin");
    if !has_plugin_flag {
        return Err("missing --plugin flag".into());
    }

    let config_idx = args.iter().position(|a| a == "--config");
    let config_b64 = config_idx
        .and_then(|i| args.get(i + 1))
        .ok_or("missing --config <base64-json> argument")?;

    let config_bytes = BASE64
        .decode(config_b64)
        .map_err(|e| format!("invalid base64 in --config: {e}"))?;

    let config: PluginConfig = serde_json::from_slice(&config_bytes)
        .map_err(|e| format!("invalid JSON in --config: {e}"))?;

    Ok(config)
}

fn run() -> Result<(), String> {
    let config = parse_args()?;

    tracing::info!(
        plugin_id = %config.plugin_id,
        data_root = %config.data_root,
        capabilities = ?config.capabilities,
        "plugin host starting"
    );

    // Initialize CapabilityGuard from the received capabilities
    let _guard = CapabilityGuard::from_names(&config.capabilities);

    // Initialize Boa context (skeleton — no plugin JS loaded yet, that's Session 3)
    let _context = boa_engine::Context::default();

    // Send "ready" signal
    let mut stdout = io::stdout().lock();
    let ready = ipc::IpcMessage::signal("init", "ready");
    ipc::write_message(&mut stdout, &ready)
        .map_err(|e| format!("failed to send ready signal: {e}"))?;

    // Enter IPC message loop (reads stdin, dispatches, writes stdout)
    let stdin = io::stdin().lock();
    let mut reader = BufReader::new(stdin);
    ipc::run_ipc_loop(&mut reader, &mut stdout).map_err(|e| format!("IPC loop error: {e}"))?;

    tracing::info!("plugin host exiting cleanly");
    Ok(())
}

fn main() {
    init_logging();
    if let Err(err) = run() {
        tracing::error!(error = %err, "volt-plugin-host exited with error");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let config = PluginConfig {
            plugin_id: "acme.test".into(),
            capabilities: vec!["fs".into(), "http".into()],
            data_root: "/tmp/plugin-data".into(),
            delegated_grants: vec![],
            host_ipc_settings: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let b64 = BASE64.encode(json.as_bytes());

        // Simulate args
        let result: PluginConfig = {
            let bytes = BASE64.decode(&b64).unwrap();
            serde_json::from_slice(&bytes).unwrap()
        };
        assert_eq!(result.plugin_id, "acme.test");
        assert_eq!(result.capabilities, vec!["fs", "http"]);
        assert_eq!(result.data_root, "/tmp/plugin-data");
    }

    #[test]
    fn test_config_camel_case_deserialization() {
        let json = r#"{"pluginId":"x.y","capabilities":[],"dataRoot":"."}"#;
        let config: PluginConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.plugin_id, "x.y");
        assert!(config.delegated_grants.is_empty());
        assert!(config.host_ipc_settings.is_none());
    }

    #[test]
    fn test_capability_guard_from_config() {
        let config = PluginConfig {
            plugin_id: "test.plugin".into(),
            capabilities: vec!["clipboard".into(), "tray".into()],
            data_root: ".".into(),
            delegated_grants: vec![],
            host_ipc_settings: None,
        };
        let guard = CapabilityGuard::from_names(&config.capabilities);
        assert!(guard.has(volt_permissions::Permission::Clipboard));
        assert!(guard.has(volt_permissions::Permission::Tray));
        assert!(!guard.has(volt_permissions::Permission::Shell));
    }

    #[test]
    fn test_config_with_delegated_grants() {
        let json = r#"{
            "pluginId": "acme.test",
            "capabilities": ["fs"],
            "dataRoot": "/tmp",
            "delegatedGrants": [
                {"grantId": "g-1", "path": "/home/user/docs"},
                {"grantId": "g-2", "path": "/home/user/pics"}
            ]
        }"#;
        let config: PluginConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.delegated_grants.len(), 2);
        assert_eq!(config.delegated_grants[0].grant_id, "g-1");
        assert_eq!(config.delegated_grants[0].path, "/home/user/docs");
        assert_eq!(config.delegated_grants[1].grant_id, "g-2");
        assert_eq!(config.delegated_grants[1].path, "/home/user/pics");
    }

    #[test]
    fn test_config_with_host_ipc_settings() {
        let json = r#"{
            "pluginId": "acme.test",
            "capabilities": [],
            "dataRoot": ".",
            "hostIpcSettings": {
                "heartbeatIntervalMs": 1000,
                "heartbeatTimeoutMs": 500,
                "callTimeoutMs": 10000,
                "maxInflight": 32,
                "maxQueueDepth": 128
            }
        }"#;
        let config: PluginConfig = serde_json::from_str(json).unwrap();
        let settings = config.host_ipc_settings.unwrap();
        assert_eq!(settings.heartbeat_interval_ms, 1000);
        assert_eq!(settings.heartbeat_timeout_ms, 500);
        assert_eq!(settings.call_timeout_ms, 10000);
        assert_eq!(settings.max_inflight, 32);
        assert_eq!(settings.max_queue_depth, 128);
    }

    #[test]
    fn test_host_ipc_settings_default() {
        let defaults = HostIpcSettings::default();
        assert_eq!(defaults.heartbeat_interval_ms, 5000);
        assert_eq!(defaults.heartbeat_timeout_ms, 3000);
        assert_eq!(defaults.call_timeout_ms, 30000);
        assert_eq!(defaults.max_inflight, 64);
        assert_eq!(defaults.max_queue_depth, 256);
    }
}
