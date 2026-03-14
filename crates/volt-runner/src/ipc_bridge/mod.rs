use std::sync::Arc;
use std::time::Duration;

use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IPC_MAX_REQUEST_BYTES, IpcResponse};

use crate::js_runtime_pool::JsRuntimePoolClient;
use crate::plugin_manager::PluginManager;

mod dispatch;
mod in_flight;
mod request_validation;
mod response;
mod worker_pool;

#[cfg(test)]
use self::dispatch::{dispatch_ipc_task, try_dispatch_native_fast_path};
use self::in_flight::InFlightTracker;
use self::request_validation::extract_request_id;
use self::response::send_response_to_window;
use self::worker_pool::{IpcDispatchTask, IpcWorkerPool};

const DEFAULT_IPC_HANDLER_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_IN_FLIGHT_PER_WINDOW: usize = 32;
const DEFAULT_MAX_IN_FLIGHT_TOTAL: usize = 128;
const DEFAULT_IPC_DISPATCH_WORKER_COUNT: usize = 4;
const IPC_IN_FLIGHT_LIMIT_CODE: &str = "IPC_IN_FLIGHT_LIMIT";

#[derive(Clone)]
pub struct IpcBridge {
    handler_timeout: Duration,
    tracker: InFlightTracker,
    worker_pool: Arc<IpcWorkerPool>,
}

impl IpcBridge {
    #[cfg(test)]
    pub fn new(runtime_client: JsRuntimePoolClient) -> Self {
        Self::new_with_plugin_manager(runtime_client, None)
    }

    pub fn new_with_plugin_manager(
        runtime_client: JsRuntimePoolClient,
        plugin_manager: Option<PluginManager>,
    ) -> Self {
        let tracker = InFlightTracker::new(
            DEFAULT_MAX_IN_FLIGHT_PER_WINDOW,
            DEFAULT_MAX_IN_FLIGHT_TOTAL,
        );
        let worker_pool = Arc::new(IpcWorkerPool::new(
            DEFAULT_IPC_DISPATCH_WORKER_COUNT,
            runtime_client,
            plugin_manager,
            tracker.clone(),
        ));

        Self {
            handler_timeout: DEFAULT_IPC_HANDLER_TIMEOUT,
            tracker,
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
                        js_window_id,
                        self.tracker.max_per_window(),
                        self.tracker.max_total()
                    ),
                    IPC_IN_FLIGHT_LIMIT_CODE.to_string(),
                    serde_json::json!({
                        "windowId": js_window_id,
                        "maxInFlightPerWindow": self.tracker.max_per_window(),
                        "maxInFlightTotal": self.tracker.max_total()
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

        if let Err(error) = self.worker_pool.enqueue(task) {
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

    fn send_response(&self, js_window_id: &str, response: IpcResponse) {
        send_response_to_window(js_window_id, response);
    }

    fn try_acquire_window_slot(&self, js_window_id: &str) -> bool {
        self.tracker.try_acquire(js_window_id)
    }

    fn release_window_slot(&self, js_window_id: &str) {
        self.tracker.release(js_window_id);
    }

    #[cfg(test)]
    fn in_flight_for(&self, js_window_id: &str) -> usize {
        self.tracker.in_flight_for(js_window_id)
    }
}

impl Drop for IpcBridge {
    fn drop(&mut self) {
        if Arc::strong_count(&self.worker_pool) == 1 {
            self.worker_pool.shutdown();
        }
    }
}

#[cfg(test)]
mod tests;
