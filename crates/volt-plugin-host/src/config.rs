use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DelegatedGrant {
    pub grant_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    pub plugin_id: String,
    pub backend_entry: String,
    pub manifest: Value,
    pub capabilities: Vec<String>,
    pub data_root: String,
    #[serde(default)]
    pub delegated_grants: Vec<DelegatedGrant>,
    #[serde(default)]
    pub host_ipc_settings: Option<HostIpcSettings>,
}

pub fn parse_args(args: &[String]) -> Result<PluginConfig, String> {
    let has_plugin_flag = args.iter().any(|argument| argument == "--plugin");
    if !has_plugin_flag {
        return Err("missing --plugin flag".into());
    }

    let config_idx = args.iter().position(|argument| argument == "--config");
    let config_b64 = config_idx
        .and_then(|index| args.get(index + 1))
        .ok_or("missing --config <base64-json> argument")?;

    let config_bytes = BASE64
        .decode(config_b64)
        .map_err(|error| format!("invalid base64 in --config: {error}"))?;

    serde_json::from_slice(&config_bytes)
        .map_err(|error| format!("invalid JSON in --config: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> Value {
        serde_json::json!({
            "id": "acme.test",
            "name": "Acme Test",
            "version": "0.1.0",
            "apiVersion": 1,
            "engine": { "volt": ">=0.1.0" },
            "backend": "./dist/plugin.js",
            "capabilities": ["fs"]
        })
    }

    #[test]
    fn parse_args_reads_camel_case_config() {
        let config = PluginConfig {
            plugin_id: "acme.test".into(),
            backend_entry: "./dist/plugin.js".into(),
            manifest: sample_manifest(),
            capabilities: vec!["fs".into()],
            data_root: ".".into(),
            delegated_grants: vec![],
            host_ipc_settings: Some(HostIpcSettings::default()),
        };
        let encoded = BASE64.encode(serde_json::to_vec(&config).expect("config json"));
        let args = vec![
            "volt-plugin-host".to_string(),
            "--plugin".to_string(),
            "--config".to_string(),
            encoded,
        ];

        let parsed = parse_args(&args).expect("parsed config");
        assert_eq!(parsed.plugin_id, "acme.test");
        assert_eq!(parsed.backend_entry, "./dist/plugin.js");
        assert_eq!(parsed.manifest["id"], "acme.test");
    }

    #[test]
    fn parse_args_rejects_missing_plugin_flag() {
        let error = parse_args(&["volt-plugin-host".to_string()]).expect_err("missing plugin flag");
        assert!(error.contains("--plugin"));
    }
}
