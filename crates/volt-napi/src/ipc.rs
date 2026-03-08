use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use volt_core::ipc::{IpcRegistry, event_script};

/// JavaScript-facing IPC handler registry.
#[napi]
pub struct VoltIpc {
    registry: Arc<IpcRegistry>,
    event_emitters: Arc<Mutex<Vec<ThreadsafeFunction<String>>>>,
}

impl Drop for VoltIpc {
    fn drop(&mut self) {
        let _ = self.registry.clear_handlers();
        if let Ok(mut emitters) = self.event_emitters.lock() {
            emitters.clear();
        }
    }
}

#[napi]
impl VoltIpc {
    /// Create a new IPC handler registry.
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        Ok(Self {
            registry: Arc::new(IpcRegistry::new()),
            event_emitters: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Register an IPC handler. The callback receives JSON args string and should
    /// return a JSON result string.
    #[napi]
    pub fn handle(
        &self,
        channel: String,
        callback: ThreadsafeFunction<String, String>,
    ) -> napi::Result<()> {
        let cb = Arc::new(callback);
        self.registry
            .register(&channel, move |args| {
                let args_json = serde_json::to_string(&args)
                    .map_err(|e| format!("Failed to serialize args: {e}"))?;

                // Request callback execution and capture the returned JSON string.
                // If the Node thread cannot service the callback, fail fast with timeout.
                let cb_clone = Arc::clone(&cb);
                let (tx, rx) = mpsc::channel();
                let status = cb_clone.call_with_return_value(
                    Ok(args_json),
                    ThreadsafeFunctionCallMode::NonBlocking,
                    move |result, _env| {
                        let callback_result =
                            result.map_err(|e| format!("IPC callback failed: {e}"));
                        let _ = tx.send(callback_result);
                        Ok(())
                    },
                );

                if status != napi::Status::Ok {
                    return Err(format!("Failed to schedule IPC callback: {status:?}"));
                }

                let callback_json = rx
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|e| format!("IPC callback timeout: {e}"))??;

                // Handlers are expected to return JSON strings, but we gracefully
                // fall back to plain string values for compatibility.
                match serde_json::from_str::<serde_json::Value>(&callback_json) {
                    Ok(value) => Ok(value),
                    Err(_) => Ok(serde_json::Value::String(callback_json)),
                }
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to register handler: {e}")))?;
        Ok(())
    }

    /// Remove a registered IPC handler.
    #[napi]
    pub fn remove_handler(&self, channel: String) -> napi::Result<()> {
        self.registry
            .remove_handler(&channel)
            .map_err(|e| napi::Error::from_reason(format!("Failed to remove handler: {e}")))?;
        Ok(())
    }

    /// Process a raw IPC message from the WebView. Returns the response JSON string.
    #[napi]
    pub fn process_message(&self, raw: String) -> napi::Result<String> {
        self.registry
            .handle_message(&raw)
            .map_err(|e| napi::Error::from_reason(format!("IPC error: {e}")))
    }

    /// Register a callback to receive event emission requests (for relaying to WebView).
    #[napi]
    pub fn on_emit(&self, callback: ThreadsafeFunction<String>) -> napi::Result<()> {
        let mut emitters = self
            .event_emitters
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock poisoned: {e}")))?;
        emitters.push(callback);
        Ok(())
    }

    /// Emit an event to all registered WebView windows.
    /// Returns the JavaScript code to evaluate in the WebView.
    #[napi]
    pub fn emit_event(&self, event: String, data: String) -> napi::Result<String> {
        let data_value: serde_json::Value = serde_json::from_str(&data)
            .map_err(|e| napi::Error::from_reason(format!("Invalid JSON data: {e}")))?;

        let script = event_script(&event, &data_value)
            .map_err(|e| napi::Error::from_reason(format!("Failed to create event script: {e}")))?;

        // Notify any registered emitter callbacks
        if let Ok(emitters) = self.event_emitters.lock() {
            for emitter in emitters.iter() {
                let status =
                    emitter.call(Ok(script.clone()), ThreadsafeFunctionCallMode::NonBlocking);
                if status != napi::Status::Ok {
                    return Err(napi::Error::from_reason(format!(
                        "Failed to dispatch IPC emit callback: {status:?}"
                    )));
                }
            }
        }

        Ok(script)
    }
}
