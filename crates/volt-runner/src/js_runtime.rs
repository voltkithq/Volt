use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value as JsonValue;
use volt_core::ipc::{IPC_HANDLER_TIMEOUT_CODE, IpcResponse};

mod bootstrap;
mod eval_ops;
mod ipc;
mod native_events;
mod requests;
mod serde_support;
#[cfg(test)]
mod tests;
mod worker;

use eval_ops::{eval_backend_bundle, eval_i64};
#[cfg(test)]
use eval_ops::{eval_bool, eval_promise_i64, eval_promise_string, eval_string, eval_unit};
use requests::RuntimeRequest;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const IPC_RESPONSE_TIMEOUT_DEFAULT: Duration = Duration::from_secs(5);
pub(crate) const IPC_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);
pub(crate) const IPC_RATE_LIMIT_MAX_REQUESTS: usize = 1000;
const JS_GC_INTERVAL: Duration = Duration::from_secs(60);
const JS_GC_REQUEST_INTERVAL: usize = 512;
const IPC_DISPATCH_SAFE_GLOBAL: &str = "__volt_ipc_dispatch_safe__";
const NATIVE_EVENT_DISPATCH_SAFE_GLOBAL: &str = "__volt_native_event_dispatch_safe__";

#[derive(Clone)]
pub struct JsRuntimeClient {
    request_tx: Sender<RuntimeRequest>,
}

pub struct JsRuntimeManager {
    client: JsRuntimeClient,
    worker_thread: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct JsRuntimeOptions {
    pub fs_base_dir: PathBuf,
    pub permissions: Vec<String>,
    pub app_name: String,
    pub secure_storage_backend: Option<String>,
    pub updater_telemetry_enabled: bool,
    pub updater_telemetry_sink: Option<String>,
}

impl Default for JsRuntimeOptions {
    fn default() -> Self {
        Self {
            fs_base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            permissions: Vec::new(),
            app_name: "Volt App".to_string(),
            secure_storage_backend: None,
            updater_telemetry_enabled: false,
            updater_telemetry_sink: None,
        }
    }
}

impl JsRuntimeManager {
    #[cfg(test)]
    pub fn start() -> Result<Self, String> {
        Self::start_with_options(JsRuntimeOptions::default())
    }

    pub fn start_with_options(options: JsRuntimeOptions) -> Result<Self, String> {
        let (request_tx, request_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();

        let worker_thread = thread::Builder::new()
            .name("volt-boa-worker".to_string())
            .spawn(move || worker::worker_main(request_rx, ready_tx, options))
            .map_err(|err| format!("failed to spawn js runtime worker thread: {err}"))?;

        let startup_result = ready_rx
            .recv_timeout(STARTUP_TIMEOUT)
            .map_err(|err| match err {
                RecvTimeoutError::Timeout => {
                    "timed out while waiting for js runtime worker startup".to_string()
                }
                RecvTimeoutError::Disconnected => {
                    "js runtime worker startup channel closed".to_string()
                }
            })?;

        if let Err(err) = startup_result {
            let _ = request_tx.send(RuntimeRequest::Shutdown);
            let _ = worker_thread.join();
            return Err(err);
        }

        Ok(Self {
            client: JsRuntimeClient { request_tx },
            worker_thread: Some(worker_thread),
        })
    }

    pub fn client(&self) -> JsRuntimeClient {
        self.client.clone()
    }
}

impl Drop for JsRuntimeManager {
    fn drop(&mut self) {
        let _ = self.client.request_tx.send(RuntimeRequest::Shutdown);
        let _ = self.worker_thread.take();
    }
}

impl JsRuntimeClient {
    pub fn eval_i64(&self, script: &str) -> Result<i64, String> {
        self.send_request(|response_tx| RuntimeRequest::EvalI64 {
            script: script.to_string(),
            response_tx,
        })
    }

    #[cfg(test)]
    pub fn eval_bool(&self, script: &str) -> Result<bool, String> {
        self.send_request(|response_tx| RuntimeRequest::EvalBool {
            script: script.to_string(),
            response_tx,
        })
    }

    #[cfg(test)]
    pub fn eval_string(&self, script: &str) -> Result<String, String> {
        self.send_request(|response_tx| RuntimeRequest::EvalString {
            script: script.to_string(),
            response_tx,
        })
    }

    #[cfg(test)]
    pub fn eval_promise_i64(&self, script: &str) -> Result<i64, String> {
        self.send_request(|response_tx| RuntimeRequest::EvalPromiseI64 {
            script: script.to_string(),
            response_tx,
        })
    }

    #[cfg(test)]
    pub fn eval_promise_string(&self, script: &str) -> Result<String, String> {
        self.send_request(|response_tx| RuntimeRequest::EvalPromiseString {
            script: script.to_string(),
            response_tx,
        })
    }

    #[cfg(test)]
    pub fn eval_unit(&self, script: &str) -> Result<(), String> {
        self.send_request(|response_tx| RuntimeRequest::EvalUnit {
            script: script.to_string(),
            response_tx,
        })
    }

    pub fn load_backend_bundle(&self, script: &str) -> Result<(), String> {
        self.send_request(|response_tx| RuntimeRequest::LoadBackendBundle {
            script: script.to_string(),
            response_tx,
        })
    }

    pub fn dispatch_ipc_message(
        &self,
        raw: &str,
        timeout: Duration,
    ) -> Result<IpcResponse, String> {
        let response_timeout = normalize_ipc_timeout(timeout);
        let (response_tx, response_rx) = mpsc::channel();
        self.request_tx
            .send(RuntimeRequest::DispatchIpc {
                raw: raw.to_string(),
                timeout: response_timeout,
                response_tx,
            })
            .map_err(|_| "js runtime worker is not running".to_string())?;

        match response_rx.recv_timeout(response_timeout) {
            Ok(response) => Ok(response),
            Err(RecvTimeoutError::Timeout) => {
                let method = serde_support::extract_ipc_method(raw);
                Ok(IpcResponse::error_with_details(
                    serde_support::extract_ipc_request_id(raw),
                    format!(
                        "IPC handler '{method}' timed out after {}ms before the runtime returned a response",
                        response_timeout.as_millis()
                    ),
                    IPC_HANDLER_TIMEOUT_CODE.to_string(),
                    serde_json::json!({
                        "timeoutMs": response_timeout.as_millis(),
                        "method": method
                    }),
                ))
            }
            Err(RecvTimeoutError::Disconnected) => {
                Err("js runtime IPC request channel disconnected".to_string())
            }
        }
    }

    pub fn dispatch_native_event(
        &self,
        event_type: &str,
        payload: JsonValue,
    ) -> Result<(), String> {
        self.request_tx
            .send(RuntimeRequest::DispatchNativeEvent {
                event_type: event_type.to_string(),
                payload,
                response_tx: None,
            })
            .map_err(|_| "js runtime worker is not running".to_string())
    }

    #[cfg(test)]
    pub fn dispatch_native_event_blocking(
        &self,
        event_type: &str,
        payload: JsonValue,
    ) -> Result<(), String> {
        let (response_tx, response_rx) = mpsc::channel();
        self.request_tx
            .send(RuntimeRequest::DispatchNativeEvent {
                event_type: event_type.to_string(),
                payload,
                response_tx: Some(response_tx),
            })
            .map_err(|_| "js runtime worker is not running".to_string())?;

        response_rx
            .recv()
            .map_err(|_| "js runtime native event channel disconnected".to_string())?
    }

    fn send_request<T>(
        &self,
        request_builder: impl FnOnce(Sender<Result<T, String>>) -> RuntimeRequest,
    ) -> Result<T, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = request_builder(response_tx);

        self.request_tx
            .send(request)
            .map_err(|_| "js runtime worker is not running".to_string())?;

        response_rx
            .recv()
            .map_err(|_| "js runtime request channel disconnected".to_string())?
    }
}

pub(crate) fn normalize_ipc_timeout(timeout: Duration) -> Duration {
    if timeout.is_zero() {
        IPC_RESPONSE_TIMEOUT_DEFAULT
    } else {
        timeout
    }
}
