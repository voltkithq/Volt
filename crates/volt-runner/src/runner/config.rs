use std::env::{self, VarError};

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
    let config_bytes = match read_override_bytes_from_env_keys(&[
        ENV_RUNNER_CONFIG_PATH,
        ENV_RUNNER_CONFIG_LEGACY,
    ])? {
        Some(bytes) => bytes,
        None => EMBEDDED_CONFIG_BYTES.to_vec(),
    };

    let mut config = parsing::parse_runner_config_bytes(&config_bytes)?;
    apply_app_name_override(&mut config, env::var(ENV_APP_NAME))?;

    Ok(config)
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
