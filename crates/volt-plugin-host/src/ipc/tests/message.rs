use crate::ipc::IpcMessage;

use super::super::message::MessageType;

use super::support::roundtrip;

#[test]
fn signal_roundtrip() {
    let msg = IpcMessage::signal("test-id", "ready");
    let parsed = roundtrip(&msg);
    assert_eq!(parsed.msg_type, MessageType::Signal);
    assert_eq!(parsed.id, "test-id");
    assert_eq!(parsed.method, "ready");
    assert!(parsed.payload.is_none());
    assert!(parsed.error.is_none());
}

#[test]
fn response_roundtrip() {
    let msg = IpcMessage::response(
        "resp-1",
        "getData",
        Some(serde_json::json!({"key": "value"})),
    );
    let parsed = roundtrip(&msg);
    assert_eq!(parsed.msg_type, MessageType::Response);
    assert_eq!(parsed.method, "getData");
    assert_eq!(parsed.payload.unwrap(), serde_json::json!({"key": "value"}));
}

#[test]
fn error_response_roundtrip() {
    let msg = IpcMessage::error_response("e-1", "doStuff", "FAILED", "something broke");
    let parsed = roundtrip(&msg);
    let err = parsed.error.unwrap();
    assert_eq!(err.code, "FAILED");
    assert_eq!(err.message, "something broke");
}

#[test]
fn request_message_serde() {
    let msg = IpcMessage {
        msg_type: MessageType::Request,
        id: "r-1".into(),
        method: "test.method".into(),
        payload: Some(serde_json::json!({"a": [1, 2, 3]})),
        error: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"request\""));
    let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.msg_type, MessageType::Request);
    assert_eq!(parsed.method, "test.method");
}

#[test]
fn event_message_serde() {
    let msg = IpcMessage {
        msg_type: MessageType::Event,
        id: "ev-1".into(),
        method: "stream:start".into(),
        payload: Some(serde_json::json!({"streamId": "s-1"})),
        error: None,
    };
    let parsed = roundtrip(&msg);
    assert_eq!(parsed.msg_type, MessageType::Event);
    assert_eq!(parsed.method, "stream:start");
}
