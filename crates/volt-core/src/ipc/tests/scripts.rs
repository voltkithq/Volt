use super::*;

#[test]
fn test_ipc_init_script_valid() {
    let script = ipc_init_script();
    assert!(script.contains("window.__volt__"));
    assert!(script.contains("window.__volt_response__"));
    assert!(script.contains("window.__volt_event__"));
    assert!(script.contains("response.errorCode"));
    assert!(script.contains("response.errorDetails"));
}

#[test]
fn test_ipc_init_script_has_abuse_bounds() {
    let script = ipc_init_script();
    assert!(script.contains("MAX_PENDING_REQUESTS"));
    assert!(script.contains("MAX_PAYLOAD_BYTES"));
    assert!(script.contains("IPC_IN_FLIGHT_LIMIT"));
    assert!(script.contains("IPC_PAYLOAD_TOO_LARGE"));
}

#[test]
fn test_response_script_escaping() {
    let script = response_script(r#"{"id":"1","result":"hello's \"world\""}"#);
    assert!(!script.contains("hello's"));
}

#[test]
fn test_event_script_generation() {
    let script = event_script("test-event", &serde_json::json!({"count": 5})).unwrap();
    assert!(script.contains("window.__volt_event__"));
    assert!(script.contains("test-event"));
    assert!(script.contains("count"));
}

#[test]
fn test_event_script_special_characters() {
    let script = event_script("user's-event", &serde_json::json!("hello")).unwrap();
    assert!(!script.contains("user's"));
    assert!(script.contains("user\\'s"));
}

#[test]
fn test_response_script_with_newlines() {
    let script = response_script("{\"id\":\"1\",\"result\":\"line1\\nline2\\rline3\"}");
    assert!(script.contains("window.__volt_response__"));
}

#[test]
fn test_response_script_escapes_js_line_separators() {
    let raw = "{\"id\":\"1\",\"result\":\"line\u{2028}sep\u{2029}end\"}";
    let script = response_script(raw);
    assert!(!script.contains('\u{2028}'));
    assert!(!script.contains('\u{2029}'));
    assert!(script.contains("\\u2028"));
    assert!(script.contains("\\u2029"));
}

#[test]
fn test_payload_too_large_response_script_preserves_request_id() {
    let script = payload_too_large_response_script(r#"{"id":"req-7","method":"demo","args":"x"}"#);
    assert!(script.contains("window.__volt_response__"));
    assert!(script.contains("req-7"));
    assert!(script.contains("IPC_PAYLOAD_TOO_LARGE"));
}
