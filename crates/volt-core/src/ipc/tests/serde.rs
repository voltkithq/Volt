use super::*;

#[test]
fn test_ipc_request_serde_roundtrip() {
    let request = IpcRequest {
        id: "req-42".to_string(),
        method: "getData".to_string(),
        args: serde_json::json!({"key": "value", "num": 123}),
    };
    let json = serde_json::to_string(&request).unwrap();
    let restored: IpcRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, "req-42");
    assert_eq!(restored.method, "getData");
    assert_eq!(restored.args["key"], "value");
    assert_eq!(restored.args["num"], 123);
}

#[test]
fn test_ipc_request_serde_default_args() {
    let request: IpcRequest = serde_json::from_str(r#"{"id":"1","method":"ping"}"#).unwrap();
    assert!(request.args.is_null());
}

#[test]
fn test_ipc_response_success_serde() {
    let response = IpcResponse::success("id-1".into(), serde_json::json!({"ok": true}));
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"result\""));
    assert!(!json.contains("\"error\""));

    let restored: IpcResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, "id-1");
    assert!(restored.result.is_some());
    assert!(restored.error.is_none());
    assert!(restored.error_code.is_none());
}

#[test]
fn test_ipc_response_error_serde() {
    let response = IpcResponse::error("id-2".into(), "something failed".into());
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"error\""));
    assert!(!json.contains("\"result\""));

    let restored: IpcResponse = serde_json::from_str(&json).unwrap();
    assert!(restored.result.is_none());
    assert_eq!(restored.error.as_deref(), Some("something failed"));
    assert_eq!(restored.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
}
