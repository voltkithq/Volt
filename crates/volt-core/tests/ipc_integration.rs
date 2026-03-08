//! Integration tests for the IPC system.
//! Tests the full request → handler → response cycle through the public API.

use volt_core::ipc::{IpcRegistry, IpcResponse};

#[test]
fn full_ipc_roundtrip_success() {
    let registry = IpcRegistry::new();

    // Register a handler that performs a "computation"
    registry
        .register("add", |args| {
            let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::json!({ "sum": a + b }))
        })
        .unwrap();

    // Send a request as raw JSON (simulating what the WebView sends)
    let raw_request = r#"{"id":"req-1","method":"add","args":{"a":10,"b":32}}"#;
    let raw_response = registry.handle_message(raw_request).unwrap();

    // Parse and verify the response
    let response: IpcResponse = serde_json::from_str(&raw_response).unwrap();
    assert_eq!(response.id, "req-1");
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["sum"], 42);
}

#[test]
fn full_ipc_roundtrip_handler_error() {
    let registry = IpcRegistry::new();

    registry
        .register("fail", |_args| Err("Something went wrong".to_string()))
        .unwrap();

    let raw_request = r#"{"id":"req-2","method":"fail","args":null}"#;
    let raw_response = registry.handle_message(raw_request).unwrap();

    let response: IpcResponse = serde_json::from_str(&raw_response).unwrap();
    assert_eq!(response.id, "req-2");
    assert!(response.result.is_none());
    assert_eq!(response.error.as_deref(), Some("Something went wrong"));
}

#[test]
fn full_ipc_roundtrip_unknown_method() {
    let registry = IpcRegistry::new();

    let raw_request = r#"{"id":"req-3","method":"nonexistent","args":{}}"#;
    let raw_response = registry.handle_message(raw_request).unwrap();

    let response: IpcResponse = serde_json::from_str(&raw_response).unwrap();
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("not found"));
}

#[test]
fn ipc_register_remove_register_cycle() {
    let registry = IpcRegistry::new();

    // Register, use, remove, re-register with different logic
    registry
        .register("counter", |_| Ok(serde_json::json!(1)))
        .unwrap();

    let r = registry
        .handle_message(r#"{"id":"1","method":"counter","args":null}"#)
        .unwrap();
    let resp: IpcResponse = serde_json::from_str(&r).unwrap();
    assert_eq!(resp.result.unwrap(), 1);

    registry.remove_handler("counter").unwrap();

    // Method should now be "not found"
    let r = registry
        .handle_message(r#"{"id":"2","method":"counter","args":null}"#)
        .unwrap();
    let resp: IpcResponse = serde_json::from_str(&r).unwrap();
    assert!(resp.error.is_some());

    // Re-register with new logic
    registry
        .register("counter", |_| Ok(serde_json::json!(999)))
        .unwrap();

    let r = registry
        .handle_message(r#"{"id":"3","method":"counter","args":null}"#)
        .unwrap();
    let resp: IpcResponse = serde_json::from_str(&r).unwrap();
    assert_eq!(resp.result.unwrap(), 999);
}

#[test]
fn ipc_prototype_pollution_blocked_in_full_pipeline() {
    let registry = IpcRegistry::new();

    registry
        .register("safe", |_| Ok(serde_json::json!("ok")))
        .unwrap();

    // Attempt prototype pollution via __proto__
    let result = registry
        .handle_message(r#"{"id":"1","method":"safe","args":{"__proto__":{"polluted":true}}}"#);
    assert!(result.is_err());

    // Attempt via constructor
    let result = registry
        .handle_message(r#"{"id":"2","method":"safe","args":{"constructor":{"prototype":{}}}}"#);
    assert!(result.is_err());

    // Normal request should still work
    let r = registry
        .handle_message(r#"{"id":"3","method":"safe","args":{"normal":"data"}}"#)
        .unwrap();
    let resp: IpcResponse = serde_json::from_str(&r).unwrap();
    assert_eq!(resp.result.unwrap(), "ok");
}
