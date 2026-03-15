use std::collections::HashMap;
use std::io::{BufWriter, Read};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use super::messaging::send_and_wait;
use crate::plugin_manager::process::io::{
    ChildPluginProcessInner, ExitState, ReadyState, spawn_exit_watcher, spawn_stderr_reader,
    spawn_stdout_reader, wait_for_exit, write_wire_message,
};
use crate::plugin_manager::process::wire::{WireMessage, WireMessageType};
use crate::plugin_manager::{
    PLUGIN_HEARTBEAT_TIMEOUT_CODE, PLUGIN_RUNTIME_ERROR_CODE, PluginProcessHandle,
    PluginRuntimeError, ProcessExitInfo,
};

const DEFAULT_EXIT_WAIT_AFTER_KILL_MS: u64 = 250;

pub(super) struct ChildPluginProcess {
    inner: Arc<ChildPluginProcessInner>,
}

impl ChildPluginProcess {
    pub(super) fn new(
        child: std::process::Child,
        stdin: std::process::ChildStdin,
        stdout: std::process::ChildStdout,
        stderr: impl Read + Send + 'static,
    ) -> Self {
        let stderr_buffer = Arc::new(Mutex::new(String::new()));
        let inner = Arc::new(ChildPluginProcessInner {
            child: Mutex::new(child),
            stdin: Mutex::new(BufWriter::new(stdin)),
            waiters: Mutex::new(HashMap::new()),
            message_listener: Mutex::new(None),
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
}

impl PluginProcessHandle for ChildPluginProcess {
    fn process_id(&self) -> Option<u32> {
        self.inner.child.lock().ok().map(|child| child.id())
    }

    fn wait_for_ready(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        self.inner.ready.wait_for_ready(&self.inner.exit, timeout)
    }

    fn activate(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = send_and_wait(
            &self.inner,
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

    fn send_event(&self, method: &str, payload: Value) -> Result<(), PluginRuntimeError> {
        crate::plugin_manager::process::io::write_wire_message(
            &mut *self.inner.stdin.lock().map_err(|_| PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: "plugin stdin is unavailable".to_string(),
            })?,
            &WireMessage {
                message_type: WireMessageType::Event,
                id: self.inner.next_id(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            },
        )
        .map_err(|error| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("failed to send plugin event: {error}"),
        })
    }

    fn request(
        &self,
        method: &str,
        payload: Value,
        timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError> {
        send_and_wait(
            &self.inner,
            WireMessage::request(self.inner.next_id(), method.to_string(), payload),
            timeout,
        )
    }

    fn heartbeat(&self, timeout: Duration) -> Result<(), PluginRuntimeError> {
        let response = send_and_wait(
            &self.inner,
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

    fn set_message_listener(
        &self,
        listener: Arc<dyn Fn(WireMessage) -> Option<WireMessage> + Send + Sync>,
    ) {
        if let Ok(mut current) = self.inner.message_listener.lock() {
            *current = Some(listener);
        }
    }

    fn stderr_snapshot(&self) -> Option<String> {
        self.inner
            .stderr
            .lock()
            .ok()
            .and_then(|stderr| (!stderr.is_empty()).then(|| stderr.clone()))
    }
}
