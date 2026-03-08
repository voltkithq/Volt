use super::bridge::{
    BridgeDispatch, build_bridge_dispatches, close_window_event_payload, ipc_message_event_payload,
    menu_event_payload, quit_event_payload, shortcut_triggered_event_payload,
};
use super::runtime::wait_for_runtime_start_with_probe;
#[cfg(not(target_os = "macos"))]
use super::runtime::{
    detach_runtime_after_startup_timeout_with, request_quit_after_startup_timeout_with_probe,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use volt_core::app::AppEvent;

fn runtime_mode_for_target(target_os: &str) -> &'static str {
    if target_os == "macos" {
        "main-thread-macos"
    } else {
        "split-runtime-threaded"
    }
}

fn current_runtime_mode() -> &'static str {
    runtime_mode_for_target(std::env::consts::OS)
}

fn load_contract_fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../../contracts/native-event-payloads.json"
    ))
    .expect("valid event contract fixtures")
}

#[test]
fn test_quit_event_payload_contract() {
    let payload = quit_event_payload();
    assert_eq!(payload.get("type").and_then(Value::as_str), Some("quit"));
    assert_eq!(payload.as_object().map(|o| o.len()), Some(1));
}

#[test]
fn test_window_closed_event_payload_contract() {
    let payload =
        close_window_event_payload("WindowId(3)".to_string(), Some("window-3".to_string()));
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("window-closed")
    );
    assert_eq!(
        payload.get("windowId").and_then(Value::as_str),
        Some("WindowId(3)")
    );
    assert_eq!(
        payload.get("jsWindowId").and_then(Value::as_str),
        Some("window-3")
    );
}

#[test]
fn test_window_closed_event_payload_contract_without_js_id() {
    let payload = close_window_event_payload("WindowId(7)".to_string(), None);
    assert!(payload.get("jsWindowId").is_some());
    assert!(payload.get("jsWindowId").unwrap().is_null());
}

#[test]
fn test_ipc_message_event_payload_contract_json_raw() {
    let payload = ipc_message_event_payload("window-1", r#"{"id":"1","method":"ping"}"#);
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("ipc-message")
    );
    assert_eq!(
        payload.get("windowId").and_then(Value::as_str),
        Some("window-1")
    );
    let raw = payload.get("raw").expect("raw field missing");
    assert_eq!(raw.get("id").and_then(Value::as_str), Some("1"));
    assert_eq!(raw.get("method").and_then(Value::as_str), Some("ping"));
}

#[test]
fn test_ipc_message_event_payload_contract_non_json_raw() {
    let payload = ipc_message_event_payload("window-2", "hello");
    assert_eq!(payload.get("raw").and_then(Value::as_str), Some("hello"));
}

#[test]
fn test_menu_event_payload_contract() {
    let payload = menu_event_payload("file-open");
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("menu-event")
    );
    assert_eq!(
        payload.get("menuId").and_then(Value::as_str),
        Some("file-open")
    );
}

#[test]
fn test_shortcut_triggered_event_payload_contract() {
    let payload = shortcut_triggered_event_payload(42);
    assert_eq!(
        payload.get("type").and_then(Value::as_str),
        Some("shortcut-triggered")
    );
    assert_eq!(payload.get("id").and_then(Value::as_u64), Some(42));
}

#[test]
fn test_build_bridge_dispatches_for_shortcut_event() {
    let mut tao_to_js = HashMap::new();
    let dispatches =
        build_bridge_dispatches(&AppEvent::ShortcutTriggered { id: 7 }, &mut tao_to_js);

    assert_eq!(dispatches.len(), 2);
    assert_eq!(dispatches[0], BridgeDispatch::ShortcutTriggered(7));
    let event_json = match &dispatches[1] {
        BridgeDispatch::EventJson(value) => value,
        _ => panic!("expected serialized event payload"),
    };
    let parsed: Value = serde_json::from_str(event_json).expect("valid event json");
    assert_eq!(
        parsed.get("type").and_then(Value::as_str),
        Some("shortcut-triggered")
    );
    assert_eq!(parsed.get("id").and_then(Value::as_u64), Some(7));
}

#[test]
fn test_build_bridge_dispatches_ignores_process_commands() {
    let mut tao_to_js = HashMap::new();
    let dispatches = build_bridge_dispatches(&AppEvent::ProcessCommands, &mut tao_to_js);
    assert!(dispatches.is_empty());
}

#[test]
fn test_wait_for_runtime_start_with_probe_success() {
    let handle = thread::spawn(|| {
        thread::sleep(Duration::from_millis(20));
        Ok::<(), String>(())
    });

    let result = wait_for_runtime_start_with_probe(
        Duration::from_millis(50),
        Duration::from_millis(5),
        || true,
        &handle,
    );
    assert!(result.is_ok());
    let _ = handle.join();
}

#[test]
fn test_wait_for_runtime_start_with_probe_timeout() {
    let handle = thread::spawn(|| Ok::<(), String>(()));

    let result = wait_for_runtime_start_with_probe(
        Duration::from_millis(30),
        Duration::from_millis(5),
        || false,
        &handle,
    );
    assert!(result.is_err());
    let _ = handle.join();
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_request_quit_after_startup_timeout_with_probe_sends_after_running() {
    let quit_requested = std::sync::atomic::AtomicBool::new(false);

    let sent = request_quit_after_startup_timeout_with_probe(
        Duration::from_millis(5),
        Duration::from_millis(1),
        || true,
        || {
            quit_requested.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        },
    );

    assert!(sent);
    assert!(quit_requested.load(std::sync::atomic::Ordering::Relaxed));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_request_quit_after_startup_timeout_with_probe_noop_when_not_running() {
    let quit_requested = std::sync::atomic::AtomicBool::new(false);
    let sent = request_quit_after_startup_timeout_with_probe(
        Duration::from_millis(8),
        Duration::from_millis(1),
        || false,
        || {
            quit_requested.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        },
    );

    assert!(!sent);
    assert!(!quit_requested.load(std::sync::atomic::Ordering::Relaxed));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_detach_runtime_after_startup_timeout_with_sends_runtime_stopped_and_shutdowns_bridge() {
    let shutdown_called = std::sync::atomic::AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();

    detach_runtime_after_startup_timeout_with(&tx, || {
        shutdown_called.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let dispatch = rx
        .recv_timeout(Duration::from_millis(20))
        .expect("runtime-stopped dispatch");
    assert!(matches!(dispatch, BridgeDispatch::RuntimeStopped));
    assert!(shutdown_called.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn test_payloads_match_contract_fixture() {
    let fixtures = load_contract_fixtures();

    assert_eq!(quit_event_payload(), fixtures["quit"]);
    assert_eq!(
        close_window_event_payload("WindowId(3)".to_string(), Some("window-3".to_string())),
        fixtures["windowClosed"]
    );
    assert_eq!(
        close_window_event_payload("WindowId(7)".to_string(), None),
        fixtures["windowClosedWithoutJsId"]
    );
    assert_eq!(
        ipc_message_event_payload("window-1", r#"{"id":"1","method":"ping"}"#),
        fixtures["ipcMessageJsonRaw"]
    );
    assert_eq!(
        ipc_message_event_payload("window-2", "hello"),
        fixtures["ipcMessageStringRaw"]
    );
    assert_eq!(menu_event_payload("file-open"), fixtures["menuEvent"]);
    assert_eq!(
        shortcut_triggered_event_payload(42),
        fixtures["shortcutTriggered"]
    );
}

#[test]
fn test_runtime_mode_for_target_mapping() {
    assert_eq!(runtime_mode_for_target("macos"), "main-thread-macos");
    assert_eq!(runtime_mode_for_target("linux"), "split-runtime-threaded");
    assert_eq!(runtime_mode_for_target("windows"), "split-runtime-threaded");
    assert_eq!(runtime_mode_for_target("freebsd"), "split-runtime-threaded");
    assert_eq!(runtime_mode_for_target("aix"), "split-runtime-threaded");
    assert_eq!(runtime_mode_for_target("android"), "split-runtime-threaded");
}

#[test]
fn test_runtime_mode_for_current_target_matches_cfg() {
    #[cfg(target_os = "macos")]
    assert_eq!(current_runtime_mode(), "main-thread-macos");
    #[cfg(not(target_os = "macos"))]
    assert_eq!(current_runtime_mode(), "split-runtime-threaded");
}
