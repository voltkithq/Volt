use std::env::{self, VarError};
use std::fs;
use std::path::PathBuf;

use volt_core::webview::WebViewConfig;
use volt_core::window::WindowConfig;

use super::RunnerError;
use super::overrides::read_override_bytes_from_env_keys;

mod parsing;
#[cfg(test)]
mod tests;

const EMBEDDED_CONFIG_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/embedded-config.json"));
const DEFAULT_WEBVIEW_URL: &str = "volt://localhost/index.html";
const ENV_RUNNER_CONFIG_PATH: &str = "VOLT_RUNNER_CONFIG_PATH";
const ENV_RUNNER_CONFIG_LEGACY: &str = "VOLT_RUNNER_CONFIG";
const ENV_APP_NAME: &str = "VOLT_APP_NAME";
const SIDECAR_RUNNER_CONFIG: &str = "volt-config.json";
const SENTINEL_RUNNER_CONFIG: &[u8; 32] = b"__VOLT_SENTINEL_RUNNER_CONFG_V1_";

#[derive(Debug, Clone)]
pub(crate) struct RunnerConfig {
    pub(crate) app_name: String,
    pub(crate) devtools: bool,
    pub(crate) permissions: Vec<String>,
    pub(crate) fs_base_dir: Option<String>,
    pub(crate) runtime_pool_size: Option<usize>,
    pub(crate) updater_telemetry_enabled: bool,
    pub(crate) updater_telemetry_sink: Option<String>,
    pub(crate) window: WindowConfig,
    pub(crate) webview: WebViewConfig,
}

pub(crate) fn load_runner_config() -> Result<RunnerConfig, RunnerError> {
    // 1. Env var override (dev/testing)
    // 2. Sidecar file alongside the exe (pre-built runner)
    // 3. Embedded bytes (compiled-in)
    let config_bytes = if let Some(bytes) = read_override_bytes_from_env_keys(&[
        ENV_RUNNER_CONFIG_PATH,
        ENV_RUNNER_CONFIG_LEGACY,
    ])? {
        bytes
    } else if let Some(bytes) = read_sidecar_config() {
        bytes
    } else {
        unwrap_sentinel_config(EMBEDDED_CONFIG_BYTES).to_vec()
    };

    let mut config = parsing::parse_runner_config_bytes(&config_bytes)?;
    apply_app_name_override(&mut config, env::var(ENV_APP_NAME))?;

    Ok(config)
}

/// If the embedded config starts with a sentinel marker, extract the actual payload.
fn unwrap_sentinel_config(bytes: &[u8]) -> &[u8] {
    if bytes.len() >= 36 && bytes[..32] == SENTINEL_RUNNER_CONFIG[..] {
        let actual_len = u32::from_le_bytes(
            bytes[32..36].try_into().unwrap_or([0; 4]),
        ) as usize;
        if actual_len == 0 {
            return &[];
        }
        let end = (36 + actual_len).min(bytes.len());
        &bytes[36..end]
    } else {
        bytes
    }
}

/// Try to read the runner config from a sidecar file alongside the current executable.
fn read_sidecar_config() -> Option<Vec<u8>> {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| fs::canonicalize(p).ok())
        .and_then(|p| p.parent().map(PathBuf::from))?;
    fs::read(exe_dir.join(SIDECAR_RUNNER_CONFIG)).ok()
}

fn apply_app_name_override(
    config: &mut RunnerConfig,
    app_name_result: Result<String, VarError>,
) -> Result<(), RunnerError> {
    match app_name_result {
        Ok(name) if !name.trim().is_empty() => {
            config.app_name = name;
        }
        Ok(_) | Err(VarError::NotPresent) => {}
        Err(VarError::NotUnicode(_)) => {
            return Err(RunnerError::Config(format!(
                "{ENV_APP_NAME} contains non-Unicode data"
            )));
        }
    }

    Ok(())
}
