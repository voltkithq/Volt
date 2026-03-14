use std::collections::HashMap;
use std::io::{BufWriter, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use super::io::{
    ChildPluginProcessInner, ExitState, ReadyState, spawn_exit_watcher, spawn_stderr_reader,
    spawn_stdout_reader, wait_for_exit, write_wire_message,
};
use super::wire::{WireMessage, WireMessageType};
use crate::plugin_manager::{
    PLUGIN_HEARTBEAT_TIMEOUT_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginBootstrapConfig,
    PluginProcessFactory, PluginProcessHandle, PluginRuntimeError, ProcessExitInfo,
};

const DEFAULT_EXIT_WAIT_AFTER_KILL_MS: u64 = 250;
const PLUGIN_HOST_PATH_ENV: &str = "VOLT_PLUGIN_HOST_PATH";

#[derive(Default)]
pub(in crate::plugin_manager) struct RealPluginProcessFactory;

struct ChildPluginProcess {
    inner: Arc<ChildPluginProcessInner>,
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
        stderr: impl Read + Send + 'static,
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

        spawn_stdout_reader(inner.clone(), stdout);
        spawn_exit_watcher(inner.clone());
        spawn_stderr_reader(stderr_buffer, stderr);

        Self { inner }
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
        self.inner.ready.wait_for_ready(&self.inner.exit, timeout)
    }

    fn activate(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = self.send_and_wait(
            WireMessage::signal(self.inner.next_id(), "activate", None),
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
            WireMessage::request(self.inner.next_id(), method.to_string(), payload),
            timeout,
        )
    }

    fn heartbeat(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = self.send_and_wait(
            WireMessage::signal(self.inner.next_id(), "heartbeat", None),
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
            &WireMessage::signal(self.inner.next_id(), "deactivate", None),
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
        self.inner.exit.set_listener(listener);
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
