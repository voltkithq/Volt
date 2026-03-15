use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::plugin_manager::process::io::{ChildPluginProcessInner, write_wire_message};
use crate::plugin_manager::process::wire::WireMessage;
use crate::plugin_manager::{PLUGIN_RUNTIME_ERROR_CODE, PluginRuntimeError};

pub(super) fn send_and_wait(
    inner: &Arc<ChildPluginProcessInner>,
    message: WireMessage,
    timeout: Duration,
) -> Result<WireMessage, PluginRuntimeError> {
    let (tx, rx) = mpsc::channel();
    inner
        .waiters
        .lock()
        .map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin waiter map is unavailable".to_string(),
        })?
        .insert(message.id.clone(), tx);
    if let Err(error) = write_wire_message(
        &mut *inner.stdin.lock().map_err(|_| PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: "plugin stdin is unavailable".to_string(),
        })?,
        &message,
    ) {
        let _ = inner
            .waiters
            .lock()
            .map(|mut waiters| waiters.remove(&message.id));
        return Err(PluginRuntimeError {
            code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
            message: format!("failed to write message to plugin host: {error}"),
        });
    }

    rx.recv_timeout(timeout).map_err(|error| {
        let _ = inner
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
