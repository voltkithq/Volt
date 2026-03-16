use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel as channel;
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IpcResponse};

use super::dispatch::dispatch_ipc_task;
use super::in_flight::InFlightTracker;
use super::response::send_response_to_window;
use crate::js_runtime_pool::JsRuntimePoolClient;
use crate::plugin_manager::PluginManager;

pub(super) struct IpcDispatchTask {
    pub(super) js_window_id: String,
    pub(super) raw: String,
    pub(super) request_id: String,
    pub(super) timeout: Duration,
}

pub(super) struct IpcWorkerPool {
    task_tx: Mutex<Option<channel::Sender<IpcDispatchTask>>>,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl IpcWorkerPool {
    pub(super) fn new(
        worker_count: usize,
        runtime_client: JsRuntimePoolClient,
        plugin_manager: Option<PluginManager>,
        tracker: InFlightTracker,
    ) -> Self {
        let (task_tx, task_rx) = channel::unbounded::<IpcDispatchTask>();
        let mut worker_handles = Vec::new();

        for worker_index in 0..worker_count.max(1) {
            let worker_name = format!("volt-ipc-bridge-{worker_index}");
            let worker_runtime_client = runtime_client.clone();
            let worker_rx = task_rx.clone();
            let worker_plugin_manager = plugin_manager.clone();
            let worker_tracker = tracker.clone();
            let worker_handle = thread::Builder::new().name(worker_name).spawn(move || {
                loop {
                    let task = match worker_rx.recv() {
                        Ok(task) => task,
                        Err(_) => return,
                    };

                    process_task(&task, &worker_tracker, || {
                        dispatch_ipc_task(
                            &worker_runtime_client,
                            worker_plugin_manager.as_ref(),
                            &task.raw,
                            &task.request_id,
                            task.timeout,
                        )
                    });
                }
            });

            if let Ok(handle) = worker_handle {
                worker_handles.push(handle);
            }
        }

        Self {
            task_tx: Mutex::new(Some(task_tx)),
            worker_handles: Mutex::new(worker_handles),
        }
    }

    pub(super) fn enqueue(&self, task: IpcDispatchTask) -> Result<(), String> {
        let sender = self
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

    pub(super) fn shutdown(&self) {
        let task_tx = match self.task_tx.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => None,
        };
        drop(task_tx);

        if let Ok(mut handles) = self.worker_handles.lock() {
            for handle in handles.drain(..) {
                let _ = handle.join();
            }
        }
    }
}

struct InFlightSlotGuard<'a> {
    tracker: &'a InFlightTracker,
    js_window_id: &'a str,
}

impl<'a> InFlightSlotGuard<'a> {
    fn new(tracker: &'a InFlightTracker, js_window_id: &'a str) -> Self {
        Self {
            tracker,
            js_window_id,
        }
    }
}

impl Drop for InFlightSlotGuard<'_> {
    fn drop(&mut self) {
        self.tracker.release(self.js_window_id);
    }
}

fn process_task(
    task: &IpcDispatchTask,
    tracker: &InFlightTracker,
    dispatch: impl FnOnce() -> IpcResponse,
) {
    let _slot_guard = InFlightSlotGuard::new(tracker, &task.js_window_id);
    let response = match panic::catch_unwind(AssertUnwindSafe(dispatch)) {
        Ok(response) => response,
        Err(payload) => {
            let panic_message = panic_payload_message(&payload);
            tracing::error!(
                request_id = %task.request_id,
                js_window_id = %task.js_window_id,
                panic = %panic_message,
                "IPC bridge worker recovered from a panicking task"
            );
            IpcResponse::error_with_code(
                task.request_id.clone(),
                format!("IPC bridge worker panicked while handling the request: {panic_message}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            )
        }
    };

    send_response_to_window(&task.js_window_id, response);
}

fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_task_releases_window_slot_when_dispatch_panics() {
        let tracker = InFlightTracker::new(1, 1);
        assert!(tracker.try_acquire("window-1"));

        process_task(
            &IpcDispatchTask {
                js_window_id: "window-1".to_string(),
                raw: "{}".to_string(),
                request_id: "req-1".to_string(),
                timeout: Duration::from_millis(5),
            },
            &tracker,
            || panic!("boom"),
        );

        assert_eq!(tracker.in_flight_for("window-1"), 0);
        assert!(tracker.try_acquire("window-1"));
    }
}
