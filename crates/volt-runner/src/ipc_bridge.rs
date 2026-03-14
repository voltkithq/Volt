use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel as channel;
use serde_json::Value as JsonValue;
use volt_core::command::{self, AppCommand};
use volt_core::ipc::{
    IPC_HANDLER_ERROR_CODE, IPC_HANDLER_TIMEOUT_CODE, IPC_MAX_REQUEST_BYTES, IpcRequest,
    IpcResponse, response_script,
};

use crate::js_runtime_pool::JsRuntimePoolClient;
use crate::modules::volt_bench;

// Generous default: handlers may open native dialogs (file picker, message box)
// that block the Boa thread while the user interacts. 5s was far too short.
const DEFAULT_IPC_HANDLER_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_MAX_IN_FLIGHT_PER_WINDOW: usize = 32;
const DEFAULT_MAX_IN_FLIGHT_TOTAL: usize = 128;
const DEFAULT_IPC_DISPATCH_WORKER_COUNT: usize = 4;
const IPC_IN_FLIGHT_LIMIT_CODE: &str = "IPC_IN_FLIGHT_LIMIT";
const IPC_PROTOTYPE_CHECK_MAX_DEPTH: usize = 64;

struct IpcDispatchTask {
    js_window_id: String,
    raw: String,
    request_id: String,
    timeout: Duration,
}

struct IpcWorkerPool {
    task_tx: Mutex<Option<channel::Sender<IpcDispatchTask>>>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct IpcBridge {
    handler_timeout: Duration,
    max_in_flight_per_window: usize,
    max_in_flight_total: usize,
    in_flight_by_window: Arc<Mutex<HashMap<String, usize>>>,
    worker_pool: Arc<IpcWorkerPool>,
}

impl IpcBridge {
    pub fn new(runtime_client: JsRuntimePoolClient) -> Self {
        let in_flight_by_window = Arc::new(Mutex::new(HashMap::new()));
        let (task_tx, task_rx) = channel::unbounded::<IpcDispatchTask>();
        let worker_pool = Arc::new(IpcWorkerPool {
            task_tx: Mutex::new(Some(task_tx)),
            worker_handles: Mutex::new(Vec::new()),
        });

        let worker_count = DEFAULT_IPC_DISPATCH_WORKER_COUNT.max(1);
        if let Ok(mut handles) = worker_pool.worker_handles.lock() {
            for worker_index in 0..worker_count {
                let worker_name = format!("volt-ipc-bridge-{worker_index}");
                let worker_runtime_client = runtime_client.clone();
                let worker_in_flight = in_flight_by_window.clone();
                let worker_rx = task_rx.clone();
                let worker_handle = thread::Builder::new().name(worker_name).spawn(move || {
                    loop {
                        let task = match worker_rx.recv() {
                            Ok(task) => task,
                            Err(_) => return,
                        };

                        let response = dispatch_ipc_task(
                            &worker_runtime_client,
                            &task.raw,
                            &task.request_id,
                            task.timeout,
                        );

                        Self::send_response_to_window(&task.js_window_id, response);
                        Self::release_window_slot_for(
                            worker_in_flight.as_ref(),
                            &task.js_window_id,
                        );
                    }
                });

                if let Ok(handle) = worker_handle {
                    handles.push(handle);
                }
            }
        }

        Self {
            handler_timeout: DEFAULT_IPC_HANDLER_TIMEOUT,
            max_in_flight_per_window: DEFAULT_MAX_IN_FLIGHT_PER_WINDOW,
            max_in_flight_total: DEFAULT_MAX_IN_FLIGHT_TOTAL,
            in_flight_by_window,
            worker_pool,
        }
    }

    pub fn handle_message(&self, js_window_id: String, raw: String) {
        let request_id = extract_request_id(&raw);
        if raw.len() > IPC_MAX_REQUEST_BYTES {
            self.send_response(
                &js_window_id,
                IpcResponse::error_with_details(
                    request_id,
                    format!(
                        "IPC payload too large ({} bytes > {} bytes)",
                        raw.len(),
                        IPC_MAX_REQUEST_BYTES
                    ),
                    "IPC_PAYLOAD_TOO_LARGE".to_string(),
                    serde_json::json!({
                        "payloadBytes": raw.len(),
                        "maxPayloadBytes": IPC_MAX_REQUEST_BYTES
                    }),
                ),
            );
            return;
        }

        if !self.try_acquire_window_slot(&js_window_id) {
            self.send_response(
                &js_window_id,
                IpcResponse::error_with_details(
                    request_id,
                    format!(
                        "IPC in-flight limit reached for window {} (window max {}, global max {})",
                        js_window_id, self.max_in_flight_per_window, self.max_in_flight_total
                    ),
                    IPC_IN_FLIGHT_LIMIT_CODE.to_string(),
                    serde_json::json!({
                        "windowId": js_window_id,
                        "maxInFlightPerWindow": self.max_in_flight_per_window,
                        "maxInFlightTotal": self.max_in_flight_total
                    }),
                ),
            );
            return;
        }

        let task = IpcDispatchTask {
            js_window_id: js_window_id.clone(),
            raw,
            request_id: request_id.clone(),
            timeout: self.handler_timeout,
        };

        if let Err(error) = self.enqueue_task(task) {
            self.release_window_slot(&js_window_id);
            self.send_response(
                &js_window_id,
                IpcResponse::error_with_code(
                    request_id,
                    format!("failed to enqueue IPC dispatch task: {error}"),
                    IPC_HANDLER_ERROR_CODE.to_string(),
                ),
            );
        }
    }

    fn enqueue_task(&self, task: IpcDispatchTask) -> Result<(), String> {
        let sender = self
            .worker_pool
            .task_tx
            .lock()
            .map_err(|_| "IPC bridge queue is unavailable".to_string())?
            .as_ref()
            .cloned()
            .ok_or_else(|| "IPC bridge is shutting down".to_string())?;

        sender
            .send(task)
            .map_err(|_| "IPC bridge worker queue is closed".to_string())
    }

    fn send_response(&self, js_window_id: &str, response: IpcResponse) {
        Self::send_response_to_window(js_window_id, response);
    }

    fn send_response_to_window(js_window_id: &str, response: IpcResponse) {
        let response_json = match serde_json::to_string(&response) {
            Ok(serialized) => serialized,
            Err(error) => {
                let fallback = IpcResponse::error_with_code(
                    response.id,
                    format!("failed to serialize IPC response: {error}"),
                    IPC_HANDLER_ERROR_CODE.to_string(),
                );
                match serde_json::to_string(&fallback) {
                    Ok(serialized) => serialized,
                    Err(_) => return,
                }
            }
        };

        let script = response_script(&response_json);
        let _ = command::send_command(AppCommand::EvaluateScript {
            js_id: js_window_id.to_string(),
            script,
        });
    }

    fn try_acquire_window_slot(&self, js_window_id: &str) -> bool {
        let mut guard = match self.in_flight_by_window.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        let in_flight = guard.get(js_window_id).copied().unwrap_or(0);
        let total_in_flight: usize = guard.values().sum();
        if total_in_flight >= self.max_in_flight_total {
            return false;
        }
        if in_flight >= self.max_in_flight_per_window {
            return false;
        }

        guard.insert(js_window_id.to_string(), in_flight + 1);
        true
    }

    fn release_window_slot(&self, js_window_id: &str) {
        Self::release_window_slot_for(self.in_flight_by_window.as_ref(), js_window_id);
    }

    fn release_window_slot_for(
        in_flight_by_window: &Mutex<HashMap<String, usize>>,
        js_window_id: &str,
    ) {
        let mut guard = match in_flight_by_window.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        match guard.get(js_window_id).copied().unwrap_or(0) {
            0 | 1 => {
                guard.remove(js_window_id);
            }
            count => {
                guard.insert(js_window_id.to_string(), count - 1);
            }
        }
    }
}

impl Drop for IpcBridge {
    fn drop(&mut self) {
        if Arc::strong_count(&self.worker_pool) != 1 {
            return;
        }

        let task_tx = match self.worker_pool.task_tx.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        drop(task_tx);

        if let Ok(mut handles) = self.worker_pool.worker_handles.lock() {
            for handle in handles.drain(..) {
                let _ = handle.join();
            }
        }
    }
}

fn extract_request_id(raw: &str) -> String {
    match serde_json::from_str::<JsonValue>(raw) {
        Ok(value) => value
            .get("id")
            .and_then(JsonValue::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_string()),
        Err(_) => "unknown".to_string(),
    }
}

fn try_dispatch_native_fast_path(raw: &str) -> Option<IpcResponse> {
    let parsed: JsonValue = serde_json::from_str(raw).ok()?;
    let request_id = extract_request_id_from_value(&parsed);

    if let Err(message) = validate_prototype_pollution(&parsed, 0) {
        return Some(IpcResponse::error_with_code(
            request_id,
            message,
            IPC_HANDLER_ERROR_CODE.to_string(),
        ));
    }

    let request: IpcRequest = match serde_json::from_value(parsed) {
        Ok(request) => request,
        Err(error) => {
            return Some(IpcResponse::error_with_code(
                request_id,
                format!("invalid IPC request: {error}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            ));
        }
    };

    if request.method == "__volt_internal:csp-violation" {
        let blocked = request
            .args
            .get("blockedURI")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let directive = request
            .args
            .get("violatedDirective")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        tracing::warn!(
            blocked_uri = blocked,
            directive = directive,
            "CSP violation: {blocked} blocked by \"{directive}\" directive. \
             Review CSP settings or check the resource URL."
        );
        return Some(IpcResponse::success(request.id, serde_json::json!(null)));
    }

    volt_bench::dispatch_native_fast_path(&request.method, request.args).map(
        |result| match result {
            Ok(payload) => IpcResponse::success(request.id, payload),
            Err(error) => {
                IpcResponse::error_with_code(request.id, error, IPC_HANDLER_ERROR_CODE.to_string())
            }
        },
    )
}

fn dispatch_ipc_task(
    runtime_client: &JsRuntimePoolClient,
    raw: &str,
    request_id: &str,
    timeout: Duration,
) -> IpcResponse {
    if let Some(response) = try_dispatch_native_fast_path(raw) {
        return match runtime_client.check_ipc_rate_limit() {
            Ok(()) => response,
            Err(error) => IpcResponse::error_with_code(
                request_id.to_string(),
                error,
                IPC_HANDLER_ERROR_CODE.to_string(),
            ),
        };
    }

    runtime_client
        .dispatch_ipc_message(raw, timeout)
        .unwrap_or_else(|error| {
            if error.contains("timed out after") {
                let method = crate::js_runtime::serde_support::extract_ipc_method(raw);
                IpcResponse::error_with_details(
                    request_id.to_string(),
                    format!("IPC bridge failure: {error}"),
                    IPC_HANDLER_TIMEOUT_CODE.to_string(),
                    serde_json::json!({
                        "timeoutMs": timeout.as_millis(),
                        "method": method
                    }),
                )
            } else {
                IpcResponse::error_with_code(
                    request_id.to_string(),
                    format!("IPC bridge failure: {error}"),
                    IPC_HANDLER_ERROR_CODE.to_string(),
                )
            }
        })
}

fn extract_request_id_from_value(value: &JsonValue) -> String {
    value
        .get("id")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string())
}

fn validate_prototype_pollution(value: &JsonValue, depth: usize) -> Result<(), String> {
    if depth > IPC_PROTOTYPE_CHECK_MAX_DEPTH {
        return Err(format!(
            "payload nesting exceeds max depth ({depth} > {IPC_PROTOTYPE_CHECK_MAX_DEPTH})"
        ));
    }

    match value {
        JsonValue::Object(map) => {
            for key in map.keys() {
                if key == "__proto__" || key == "constructor" || key == "prototype" {
                    return Err("prototype pollution attempt detected".to_string());
                }
            }
            for nested in map.values() {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        JsonValue::Array(array) => {
            for nested in array {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests;
