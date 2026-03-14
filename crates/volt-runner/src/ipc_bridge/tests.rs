use super::*;
use crate::js_runtime::JsRuntimeOptions;
use crate::js_runtime_pool::JsRuntimePool;
use serde_json::json;
use std::time::Duration;
use volt_core::ipc::IPC_MAX_REQUEST_BYTES;

#[test]
fn enforces_per_window_in_flight_limit() {
    let runtime =
        JsRuntimePool::start_with_options(2, JsRuntimeOptions::default()).expect("runtime");
    let bridge = IpcBridge::new(runtime.client());
    let window_id = "window-1";

    for _ in 0..DEFAULT_MAX_IN_FLIGHT_PER_WINDOW {
        assert!(bridge.try_acquire_window_slot(window_id));
    }
    assert!(!bridge.try_acquire_window_slot(window_id));

    for _ in 0..DEFAULT_MAX_IN_FLIGHT_PER_WINDOW {
        bridge.release_window_slot(window_id);
    }
    assert!(bridge.try_acquire_window_slot(window_id));
}

#[test]
fn enforces_global_in_flight_limit() {
    let runtime =
        JsRuntimePool::start_with_options(2, JsRuntimeOptions::default()).expect("runtime");
    let bridge = IpcBridge::new(runtime.client());

    let mut window_ids = Vec::new();
    for index in 0..DEFAULT_MAX_IN_FLIGHT_TOTAL {
        let window_id = format!("window-{index}");
        assert!(bridge.try_acquire_window_slot(window_id.as_str()));
        window_ids.push(window_id);
    }

    assert!(!bridge.try_acquire_window_slot("overflow-window"));

    for window_id in &window_ids {
        bridge.release_window_slot(window_id.as_str());
    }
    assert!(bridge.try_acquire_window_slot("overflow-window"));
}

#[test]
fn oversized_payload_is_rejected_before_it_consumes_in_flight_capacity() {
    let runtime =
        JsRuntimePool::start_with_options(2, JsRuntimeOptions::default()).expect("runtime");
    let bridge = IpcBridge::new(runtime.client());
    let oversized = format!(
        r#"{{"id":"too-big","method":"echo","args":"{}"}}"#,
        "x".repeat(IPC_MAX_REQUEST_BYTES + 1)
    );

    bridge.handle_message("window-oversized".to_string(), oversized);

    let in_flight = bridge
        .in_flight_by_window
        .lock()
        .expect("in-flight map")
        .get("window-oversized")
        .copied()
        .unwrap_or(0);
    assert_eq!(in_flight, 0);
}

#[test]
fn direct_fast_path_handles_reserved_data_query_channel() {
    let response = try_dispatch_native_fast_path(
        &json!({
            "id": "direct-query",
            "method": "volt:native:data.query",
            "args": {
                "datasetSize": 1200,
                "iterations": 2,
                "searchTerm": "risk"
            }
        })
        .to_string(),
    )
    .expect("direct fast-path response");

    assert_eq!(response.id, "direct-query");
    assert!(response.error.is_none());
    assert_eq!(
        response.result.as_ref().expect("result payload")["datasetSize"],
        json!(1200)
    );
}

#[test]
fn direct_fast_path_rejects_prototype_pollution_payloads() {
    let response = try_dispatch_native_fast_path(
        &json!({
            "id": "direct-prototype",
            "method": "volt:native:data.profile",
            "args": {
                "__proto__": {
                    "polluted": true
                }
            }
        })
        .to_string(),
    )
    .expect("direct fast-path response");

    assert_eq!(response.id, "direct-prototype");
    assert_eq!(response.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
    assert!(
        response
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("prototype pollution")
    );
}

#[test]
fn direct_fast_path_respects_existing_rate_limit() {
    let runtime =
        JsRuntimePool::start_with_options(2, JsRuntimeOptions::default()).expect("runtime");
    let client = runtime.client();
    client.fill_ipc_rate_limit_for_tests();

    let response = dispatch_ipc_task(
        &client,
        None,
        &json!({
            "id": "direct-rate-limit",
            "method": "volt:native:data.profile",
            "args": {
                "datasetSize": 1200
            }
        })
        .to_string(),
        "direct-rate-limit",
        Duration::from_secs(1),
    );

    assert_eq!(response.id, "direct-rate-limit");
    assert_eq!(response.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
    assert!(
        response
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("rate limit exceeded")
    );
}
