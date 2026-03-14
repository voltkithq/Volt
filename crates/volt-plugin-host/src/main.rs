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
use volt_core::permissions::CapabilityGuard;

/// Configuration received from the host process via `--config <base64-json>`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub plugin_id: String,
    pub capabilities: Vec<String>,
    pub data_root: String,
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
    }

    #[test]
    fn test_capability_guard_from_config() {
        let config = PluginConfig {
            plugin_id: "test.plugin".into(),
            capabilities: vec!["clipboard".into(), "tray".into()],
            data_root: ".".into(),
        };
        let guard = CapabilityGuard::from_names(&config.capabilities);
        assert!(guard.has(volt_core::permissions::Permission::Clipboard));
        assert!(guard.has(volt_core::permissions::Permission::Tray));
        assert!(!guard.has(volt_core::permissions::Permission::Shell));
    }
}
