//! Async dialog infrastructure.
//!
//! Native dialog calls (file picker, save dialog, message box) block the calling
//! thread. Since Boa native functions run on the JS worker thread, blocking there
//! stalls the entire IPC dispatch pipeline for that worker.
//!
//! This module provides a mechanism to run dialogs on a dedicated thread and
//! resolve the corresponding JS promise when the result is ready. The flow:
//!
//! 1. Native function creates a pending `JsPromise` via `JsPromise::new_pending`.
//! 2. The resolve/reject `JsFunction` handles are stored in thread-local state
//!    alongside a channel receiver.
//! 3. The blocking `rfd` call runs on a spawned thread. When done, it sends the
//!    result through the channel.
//! 4. The IPC job drain loop calls `settle_pending_dialogs()` each iteration,
//!    which checks the channel and resolves/rejects completed promises.

use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use boa_engine::object::builtins::{JsFunction, JsPromise};
use boa_engine::{Context, JsValue, js_string};
use serde_json::Value as JsonValue;

/// A pending dialog whose result will arrive through a channel.
struct PendingDialog {
    receiver: mpsc::Receiver<Result<JsonValue, String>>,
    resolve: JsFunction,
    reject: JsFunction,
}

thread_local! {
    static PENDING_DIALOGS: RefCell<Vec<PendingDialog>> = const { RefCell::new(Vec::new()) };
}

/// Create a pending JS promise and schedule the blocking `dialog_fn` on a
/// separate thread. When the dialog completes, calling `settle_pending_dialogs`
/// will resolve or reject the promise.
///
/// Returns the pending `JsPromise` to be returned from the native function.
pub fn spawn_dialog<F>(context: &mut Context, dialog_fn: F) -> JsPromise
where
    F: FnOnce() -> Result<JsonValue, String> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    let (promise, resolvers) = JsPromise::new_pending(context);

    PENDING_DIALOGS.with(|dialogs| {
        dialogs.borrow_mut().push(PendingDialog {
            receiver: rx,
            resolve: resolvers.resolve,
            reject: resolvers.reject,
        });
    });

    thread::Builder::new()
        .name("volt-dialog".to_string())
        .spawn(move || {
            let result = dialog_fn();
            let _ = tx.send(result);
        })
        .expect("failed to spawn dialog thread");

    promise
}

/// Check for completed dialog results and resolve/reject their promises.
/// Called from the IPC job drain loop on each iteration.
pub fn settle_pending_dialogs(context: &mut Context) {
    PENDING_DIALOGS.with(|dialogs| {
        let mut pending = dialogs.borrow_mut();
        let mut i = 0;
        while i < pending.len() {
            match pending[i].receiver.try_recv() {
                Ok(result) => {
                    let dialog = pending.swap_remove(i);
                    match result {
                        Ok(json_value) => {
                            if let Ok(js_val) = JsValue::from_json(&json_value, context) {
                                let _ =
                                    dialog
                                        .resolve
                                        .call(&JsValue::undefined(), &[js_val], context);
                            }
                        }
                        Err(message) => {
                            let err_val = JsValue::from(js_string!(message.as_str()));
                            let _ = dialog
                                .reject
                                .call(&JsValue::undefined(), &[err_val], context);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    i += 1;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    let dialog = pending.swap_remove(i);
                    let err_val =
                        JsValue::from(js_string!("dialog thread terminated unexpectedly"));
                    let _ = dialog
                        .reject
                        .call(&JsValue::undefined(), &[err_val], context);
                }
            }
        }
    });
}

/// Returns true if there are pending dialogs waiting for results.
pub fn has_pending_dialogs() -> bool {
    PENDING_DIALOGS.with(|dialogs| !dialogs.borrow().is_empty())
}
