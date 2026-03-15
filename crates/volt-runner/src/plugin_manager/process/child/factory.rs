use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use super::handle::ChildPluginProcess;
use crate::plugin_manager::{
    PLUGIN_RUNTIME_ERROR_CODE, PluginBootstrapConfig, PluginProcessFactory, PluginProcessHandle,
    PluginRuntimeError,
};

const PLUGIN_HOST_PATH_ENV: &str = "VOLT_PLUGIN_HOST_PATH";

#[derive(Default)]
pub(crate) struct RealPluginProcessFactory;

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
