use super::*;

#[test]
fn test_register_and_handle() {
    let registry = IpcRegistry::new();
    registry
        .register("greet", |args| {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("World");
            Ok(serde_json::json!({ "message": format!("Hello, {name}!") }))
        })
        .unwrap();

    let response = registry
        .handle_message(r#"{"id":"1","method":"greet","args":{"name":"Volt"}}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed.id, "1");
    assert!(parsed.error.is_none());
    assert_eq!(parsed.result.unwrap()["message"], "Hello, Volt!");
}

#[test]
fn test_handler_not_found() {
    let registry = IpcRegistry::new();
    let response = registry
        .handle_message(r#"{"id":"2","method":"nonexistent","args":{}}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert!(parsed.error.is_some());
    assert!(parsed.error.unwrap().contains("not found"));
    assert_eq!(
        parsed.error_code.as_deref(),
        Some(IPC_HANDLER_NOT_FOUND_CODE)
    );
}

#[test]
fn test_prototype_pollution_rejected() {
    let registry = IpcRegistry::new();
    let result =
        registry.handle_message(r#"{"id":"3","method":"test","args":{"__proto__":{"evil":true}}}"#);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IpcError::PrototypePollution));
}

#[test]
fn test_constructor_pollution_rejected() {
    let registry = IpcRegistry::new();
    let result = registry
        .handle_message(r#"{"id":"4","method":"test","args":{"constructor":{"prototype":{}}}}"#);
    assert!(result.is_err());
}

#[test]
fn test_prototype_keywords_in_string_values_allowed() {
    let registry = IpcRegistry::new();
    registry
        .register("echo", Ok)
        .expect("register echo handler");

    let response = registry
        .handle_message(
            r#"{"id":"5","method":"echo","args":{"message":"constructor prototype __proto__ are plain text"}}"#,
        )
        .expect("string values should not be rejected");
    let parsed: IpcResponse = serde_json::from_str(&response).expect("deserialize response");
    assert!(parsed.error.is_none());
    assert_eq!(
        parsed.result.as_ref().and_then(|r| r.get("message")),
        Some(&serde_json::Value::String(
            "constructor prototype __proto__ are plain text".to_string()
        ))
    );
}

#[test]
fn test_invalid_json_rejected() {
    let registry = IpcRegistry::new();
    let result = registry.handle_message("not valid json");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IpcError::InvalidMessage(_)));
}

#[test]
fn test_payload_size_limit() {
    let registry = IpcRegistry::new();
    let oversized = "x".repeat(IPC_MAX_REQUEST_BYTES + 1);
    let result = registry.handle_message(&oversized);
    assert!(matches!(
        result,
        Err(IpcError::PayloadTooLarge {
            size: _,
            max: IPC_MAX_REQUEST_BYTES
        })
    ));
}

#[test]
fn test_rate_limiter() {
    let mut limiter = RateLimiter::new(3, Duration::from_secs(1));
    assert!(limiter.check().is_ok());
    assert!(limiter.check().is_ok());
    assert!(limiter.check().is_ok());
    assert!(limiter.check().is_err());
}

#[test]
fn test_remove_handler() {
    let registry = IpcRegistry::new();
    registry
        .register("temp", |_| Ok(serde_json::json!(null)))
        .unwrap();

    let response = registry
        .handle_message(r#"{"id":"1","method":"temp","args":null}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert!(parsed.error.is_none());

    registry.remove_handler("temp").unwrap();
    let response = registry
        .handle_message(r#"{"id":"2","method":"temp","args":null}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert!(parsed.error.is_some());
    assert_eq!(
        parsed.error_code.as_deref(),
        Some(IPC_HANDLER_NOT_FOUND_CODE)
    );
}

#[test]
fn test_clear_handlers() {
    let registry = IpcRegistry::new();
    registry
        .register("one", |_| Ok(serde_json::json!(1)))
        .unwrap();
    registry
        .register("two", |_| Ok(serde_json::json!(2)))
        .unwrap();

    registry.clear_handlers().unwrap();

    let response = registry
        .handle_message(r#"{"id":"3","method":"one","args":null}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert_eq!(
        parsed.error_code.as_deref(),
        Some(IPC_HANDLER_NOT_FOUND_CODE)
    );
}

#[test]
fn test_handler_that_returns_error() {
    let registry = IpcRegistry::new();
    registry
        .register("fail", |_| Err("intentional error".to_string()))
        .unwrap();

    let response = registry
        .handle_message(r#"{"id":"1","method":"fail","args":null}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert_eq!(parsed.error.as_deref(), Some("intentional error"));
    assert!(parsed.result.is_none());
    assert_eq!(parsed.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
}

#[test]
fn test_prototype_pollution_deeply_nested() {
    let registry = IpcRegistry::new();
    let result = registry
        .handle_message(r#"{"id":"1","method":"test","args":{"a":{"b":{"c":{"__proto__":{}}}}}}"#);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IpcError::PrototypePollution));
}

#[test]
fn test_prototype_pollution_in_array() {
    let registry = IpcRegistry::new();
    let result = registry
        .handle_message(r#"{"id":"1","method":"test","args":{"items":[{"__proto__":{}}]}}"#);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), IpcError::PrototypePollution));
}

#[test]
fn test_prototype_pollution_depth_limit() {
    let registry = IpcRegistry::new();
    let mut nested = String::from("{}");
    for _ in 0..70 {
        nested = format!(r#"{{"layer":{nested}}}"#);
    }
    let payload = format!(r#"{{"id":"1","method":"test","args":{nested}}}"#);
    let result = registry.handle_message(&payload);
    assert!(matches!(result, Err(IpcError::Security(_))));
}

#[test]
fn test_rate_limiter_window_expiry() {
    let mut limiter = RateLimiter::new(2, Duration::from_millis(50));
    assert!(limiter.check().is_ok());
    assert!(limiter.check().is_ok());
    assert!(limiter.check().is_err());

    std::thread::sleep(Duration::from_millis(60));
    assert!(limiter.check().is_ok());
}

#[test]
fn test_registry_default() {
    let registry = IpcRegistry::default();
    let response = registry
        .handle_message(r#"{"id":"1","method":"nope","args":null}"#)
        .unwrap();
    let parsed: IpcResponse = serde_json::from_str(&response).unwrap();
    assert!(parsed.error.is_some());
    assert_eq!(
        parsed.error_code.as_deref(),
        Some(IPC_HANDLER_NOT_FOUND_CODE)
    );
}
