use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Read};
use std::process::{Child, ChildStdin, ChildStdout};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use super::stderr_capture::read_bounded_stderr;
use super::wire::{WireMessage, WireMessageType};
use super::wire_io::read_wire_message;
use crate::plugin_manager::{ExitListener, MessageListener, ProcessExitInfo};

pub(super) struct ChildPluginProcessInner {
    pub(super) child: Mutex<Child>,
    pub(super) stdin: Mutex<BufWriter<ChildStdin>>,
    pub(super) waiters: Mutex<HashMap<String, mpsc::Sender<WireMessage>>>,
    pub(super) message_listener: Mutex<Option<MessageListener>>,
    pub(super) ready: ReadyState,
    pub(super) exit: ExitState,
    pub(super) next_id: AtomicU64,
    pub(super) stderr: Arc<Mutex<String>>,
}

pub(super) struct ReadyState {
    ready: Mutex<bool>,
    condvar: Condvar,
}

pub(super) struct ExitState {
    pub(super) info: Mutex<Option<ProcessExitInfo>>,
    condvar: Condvar,
    listener: Mutex<Option<ExitListener>>,
}

impl Default for ReadyState {
    fn default() -> Self {
        Self {
            ready: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }
}

impl Default for ExitState {
    fn default() -> Self {
        Self {
            info: Mutex::new(None),
            condvar: Condvar::new(),
            listener: Mutex::new(None),
        }
    }
}

impl ChildPluginProcessInner {
    pub(super) fn next_id(&self) -> String {
        format!("plugin-{}", self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}

impl ReadyState {
    pub(super) fn wait_for_ready(
        &self,
        exit_state: &ExitState,
        timeout: Duration,
    ) -> Result<(), crate::plugin_manager::PluginRuntimeError> {
        let mut ready =
            self.ready
                .lock()
                .map_err(|_| crate::plugin_manager::PluginRuntimeError {
                    code: crate::plugin_manager::PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "plugin ready state is unavailable".to_string(),
                })?;
        if *ready {
            return Ok(());
        }

        let deadline = std::time::Instant::now() + timeout;
        loop {
            if exit_state
                .info
                .lock()
                .ok()
                .and_then(|info| info.clone())
                .is_some()
            {
                return Err(crate::plugin_manager::PluginRuntimeError {
                    code: crate::plugin_manager::PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "plugin exited before sending ready".to_string(),
                });
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                return Err(crate::plugin_manager::PluginRuntimeError {
                    code: "TIMEOUT".to_string(),
                    message: format!("plugin did not send ready within {}ms", timeout.as_millis()),
                });
            }
            let (next_ready, _) = self
                .condvar
                .wait_timeout(ready, deadline.saturating_duration_since(now))
                .map_err(|_| crate::plugin_manager::PluginRuntimeError {
                    code: crate::plugin_manager::PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "plugin ready wait failed".to_string(),
                })?;
            ready = next_ready;
            if *ready {
                return Ok(());
            }
        }
    }

    fn mark_ready(&self) {
        if let Ok(mut ready) = self.ready.lock() {
            *ready = true;
            self.condvar.notify_all();
        }
    }
}

impl ExitState {
    pub(super) fn set_listener(&self, listener: ExitListener) {
        if let Some(exit) = self.info.lock().ok().and_then(|info| info.clone()) {
            listener(exit);
            return;
        }
        if let Ok(mut current) = self.listener.lock() {
            *current = Some(listener);
        }
    }
}

pub(super) fn spawn_stdout_reader(process: Arc<ChildPluginProcessInner>, stdout: ChildStdout) {
    let _ = thread::Builder::new()
        .name("volt-plugin-host-stdout".to_string())
        .spawn(move || read_plugin_stdout(process, stdout));
}

pub(super) fn spawn_exit_watcher(process: Arc<ChildPluginProcessInner>) {
    let _ = thread::Builder::new()
        .name("volt-plugin-host-exit".to_string())
        .spawn(move || {
            let exit_code = process
                .child
                .lock()
                .ok()
                .and_then(|mut child| child.wait().ok())
                .and_then(|status| status.code());
            drain_waiters(&process.waiters);
            notify_exit(&process.exit, ProcessExitInfo { code: exit_code });
        });
}

pub(super) fn spawn_stderr_reader(
    stderr_buffer: Arc<Mutex<String>>,
    mut stderr: impl Read + Send + 'static,
) {
    let _ = thread::Builder::new()
        .name("volt-plugin-host-stderr".to_string())
        .spawn(move || {
            let captured = read_bounded_stderr(&mut stderr);
            if let Ok(mut buffer) = stderr_buffer.lock() {
                *buffer = captured;
            }
        });
}

pub(super) fn wait_for_exit(exit_state: &ExitState, timeout: Duration) -> Option<ProcessExitInfo> {
    let mut info = exit_state.info.lock().ok()?;
    if info.is_some() {
        return info.clone();
    }
    let (next_info, _) = exit_state.condvar.wait_timeout(info, timeout).ok()?;
    info = next_info;
    info.clone()
}

fn read_plugin_stdout(process: Arc<ChildPluginProcessInner>, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout);
    loop {
        let message = match read_wire_message(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) | Err(_) => {
                drain_waiters(&process.waiters);
                return;
            }
        };

        if message.message_type == WireMessageType::Signal && message.method == "ready" {
            process.ready.mark_ready();
            continue;
        }

        if let Ok(mut waiters) = process.waiters.lock()
            && let Some(waiter) = waiters.remove(&message.id)
        {
            let _ = waiter.send(message);
            continue;
        }

        let listener = process
            .message_listener
            .lock()
            .ok()
            .and_then(|listener| listener.clone());
        if let Some(listener) = listener
            && let Some(response) = listener(message)
        {
            let _ = process
                .stdin
                .lock()
                .ok()
                .map(|mut stdin| super::wire_io::write_wire_message(&mut *stdin, &response));
        }
    }
}

fn notify_exit(exit_state: &ExitState, exit: ProcessExitInfo) {
    if let Ok(mut info) = exit_state.info.lock()
        && info.is_none()
    {
        *info = Some(exit.clone());
    }
    exit_state.condvar.notify_all();
    if let Ok(listener) = exit_state.listener.lock()
        && let Some(listener) = listener.clone()
    {
        listener(exit);
    }
}

pub(super) fn drain_waiters(waiters: &Mutex<HashMap<String, mpsc::Sender<WireMessage>>>) {
    if let Ok(mut waiters) = waiters.lock() {
        waiters.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_waiters_disconnects_pending_receivers() {
        let waiters = Mutex::new(HashMap::new());
        let (tx, rx) = mpsc::channel();
        waiters
            .lock()
            .expect("waiters lock")
            .insert("req-1".to_string(), tx);

        drain_waiters(&waiters);

        assert!(matches!(rx.recv(), Err(mpsc::RecvError)));
        assert!(waiters.lock().expect("waiters lock").is_empty());
    }
}
