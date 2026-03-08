use crate::global_shortcut::dispatch_shortcut_trigger;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use volt_core::app::AppEvent;

static DROPPED_EVENT_DISPATCHES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, PartialEq)]
pub(super) enum BridgeDispatch {
    EventJson(String),
    ShortcutTriggered(u32),
    RuntimeStopped,
}

pub(super) fn spawn_bridge_thread(
    callbacks: Arc<Mutex<Vec<ThreadsafeFunction<String>>>>,
    dispatch_rx: mpsc::Receiver<BridgeDispatch>,
) -> napi::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("volt-node-bridge".to_string())
        .spawn(move || run_bridge_dispatch_loop(callbacks, dispatch_rx))
        .map_err(|e| napi::Error::from_reason(format!("Failed to spawn bridge thread: {e}")))
}

fn run_bridge_dispatch_loop(
    callbacks: Arc<Mutex<Vec<ThreadsafeFunction<String>>>>,
    dispatch_rx: mpsc::Receiver<BridgeDispatch>,
) {
    while let Ok(dispatch) = dispatch_rx.recv() {
        match dispatch {
            BridgeDispatch::EventJson(event_json) => {
                if let Ok(cbs) = callbacks.lock() {
                    for cb in cbs.iter() {
                        let status = cb.call(
                            Ok(event_json.clone()),
                            ThreadsafeFunctionCallMode::NonBlocking,
                        );
                        if status != napi::Status::Ok {
                            DROPPED_EVENT_DISPATCHES.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            BridgeDispatch::ShortcutTriggered(id) => {
                dispatch_shortcut_trigger(id);
            }
            BridgeDispatch::RuntimeStopped => {
                break;
            }
        }
    }

    if let Ok(mut callbacks) = callbacks.lock() {
        callbacks.clear();
    }
    crate::global_shortcut::clear_shortcut_callbacks();
    let dropped = DROPPED_EVENT_DISPATCHES.swap(0, Ordering::Relaxed);
    if dropped > 0 {
        tracing::warn!(
            dropped_dispatches = dropped,
            "dropped native event callback dispatches"
        );
    }
}

pub(super) fn build_bridge_dispatches(
    event: &AppEvent,
    tao_to_js: &mut HashMap<String, String>,
) -> Vec<BridgeDispatch> {
    match event {
        AppEvent::Quit => vec![BridgeDispatch::EventJson(quit_event_payload().to_string())],
        AppEvent::CloseWindow(id) => {
            let window_key = window_key(id);
            let js_window_id = tao_to_js.remove(&window_key);
            let window_id = js_window_id.clone().unwrap_or(window_key);
            vec![BridgeDispatch::EventJson(
                close_window_event_payload(window_id, js_window_id).to_string(),
            )]
        }
        AppEvent::IpcMessage { js_window_id, raw } => vec![BridgeDispatch::EventJson(
            ipc_message_event_payload(js_window_id, raw).to_string(),
        )],
        AppEvent::MenuEvent { menu_id } => {
            vec![BridgeDispatch::EventJson(
                menu_event_payload(menu_id).to_string(),
            )]
        }
        AppEvent::ShortcutTriggered { id } => vec![
            BridgeDispatch::ShortcutTriggered(*id),
            BridgeDispatch::EventJson(shortcut_triggered_event_payload(*id).to_string()),
        ],
        AppEvent::ProcessCommands => Vec::new(),
        _ => Vec::new(),
    }
}

fn window_key(id: &impl Hash) -> String {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    format!("native-window-{:016x}", hasher.finish())
}

pub(super) fn quit_event_payload() -> Value {
    json!({ "type": "quit" })
}

pub(super) fn close_window_event_payload(window_id: String, js_window_id: Option<String>) -> Value {
    json!({
        "type": "window-closed",
        "windowId": window_id,
        "jsWindowId": js_window_id,
    })
}

pub(super) fn ipc_message_event_payload(js_window_id: &str, raw: &str) -> Value {
    let parsed_raw =
        serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
    json!({
        "type": "ipc-message",
        "windowId": js_window_id,
        "raw": parsed_raw,
    })
}

pub(super) fn menu_event_payload(menu_id: &str) -> Value {
    json!({
        "type": "menu-event",
        "menuId": menu_id,
    })
}

pub(super) fn shortcut_triggered_event_payload(id: u32) -> Value {
    json!({
        "type": "shortcut-triggered",
        "id": id,
    })
}
