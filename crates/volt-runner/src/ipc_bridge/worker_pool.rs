use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel as channel;

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

                    let js_window_id = task.js_window_id.clone();

                    // Catch panics to guarantee the in-flight slot is always
                    // released, preventing permanent slot exhaustion.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let response = dispatch_ipc_task(
                            &worker_runtime_client,
                            worker_plugin_manager.as_ref(),
                            &task.raw,
                            &task.request_id,
                            task.timeout,
                        );
                        send_response_to_window(&task.js_window_id, response);
                    }));

                    if result.is_err() {
                        tracing::error!(
                            window = %js_window_id,
                            "IPC dispatch panicked — slot released, no response sent"
                        );
                    }

                    worker_tracker.release(&js_window_id);
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
