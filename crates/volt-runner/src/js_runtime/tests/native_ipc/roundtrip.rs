use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;
use crate::js_runtime::requests::RuntimeRequest;

#[test]
fn ipc_roundtrip_supports_sync_and_async_handlers() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    ipcMain.handle('sum', (args) => args.a + args.b);
                    ipcMain.handle('delayed', async (args) => {
                        await new Promise((resolve) => setTimeout(resolve, 5));
                        return { ok: true, value: args.value };
                    });
                    return 'registered';
                })()",
        )
        .expect("register ipc handlers");

    let sync = dispatch_ipc_request(
        &client,
        r#"{"id":"sync-1","method":"sum","args":{"a":20,"b":22}}"#,
    );
    assert_eq!(sync.id, "sync-1");
    assert_eq!(sync.result, Some(serde_json::json!(42)));
    assert!(sync.error.is_none());

    let async_response = dispatch_ipc_request(
        &client,
        r#"{"id":"async-1","method":"delayed","args":{"value":"ok"}}"#,
    );
    assert_eq!(async_response.id, "async-1");
    assert_eq!(
        async_response.result,
        Some(serde_json::json!({ "ok": true, "value": "ok" }))
    );
    assert!(async_response.error.is_none());
}

#[test]
fn ipc_main_rejects_reserved_volt_channels() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    let error = client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    ipcMain.handle('volt:native:data.query', () => ({ ok: true }));
                    return 'unreachable';
                })()",
        )
        .expect_err("reserved channel should be rejected");

    assert!(error.contains("reserved by Volt"));
}

#[test]
fn ipc_roundtrip_handles_reserved_native_fast_path_without_js_handler() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let response = dispatch_ipc_request(
        &runtime.client(),
        r#"{"id":"native-direct-1","method":"volt:native:data.query","args":{"datasetSize":1200,"iterations":2,"searchTerm":"risk"}}"#,
    );

    assert_eq!(response.id, "native-direct-1");
    assert!(response.error.is_none());
    assert_eq!(
        response.result.as_ref().expect("native result")["datasetSize"],
        serde_json::json!(1200)
    );
}

#[test]
fn ipc_roundtrip_returns_not_found_error() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let response = dispatch_ipc_request(
        &runtime.client(),
        r#"{"id":"missing-1","method":"nope","args":{}}"#,
    );

    assert_eq!(response.id, "missing-1");
    assert!(response.result.is_none());
    assert!(
        response
            .error
            .as_deref()
            .is_some_and(|message| message.contains("Handler not found"))
    );
    assert_eq!(
        response.error_code.as_deref(),
        Some(volt_core::ipc::IPC_HANDLER_NOT_FOUND_CODE)
    );
}

#[test]
fn ipc_roundtrip_times_out_long_running_handlers() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    ipcMain.handle('slow', async () => {
                        await new Promise((resolve) => setTimeout(resolve, 50));
                        return 'done';
                    });
                    return 'registered';
                })()",
        )
        .expect("register slow handler");

    let response = client
        .dispatch_ipc_message(
            r#"{"id":"timeout-1","method":"slow","args":null}"#,
            Duration::from_millis(5),
        )
        .expect("dispatch timeout");

    assert_eq!(response.id, "timeout-1");
    assert!(response.result.is_none());
    assert_eq!(
        response.error_code.as_deref(),
        Some(IPC_HANDLER_TIMEOUT_CODE)
    );
}

#[test]
fn ipc_dispatch_timeout_is_end_to_end_for_sync_handlers() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    ipcMain.handle('busy-sync', () => {
                        const startedAt = Date.now();
                        while ((Date.now() - startedAt) < 50) {
                        }
                        return 'done';
                    });
                    return 'registered';
                })()",
        )
        .expect("register busy handler");

    let response = client
        .dispatch_ipc_message(
            r#"{"id":"timeout-sync-1","method":"busy-sync","args":null}"#,
            Duration::from_millis(5),
        )
        .expect("sync handler timeout should return a structured response");
    assert_eq!(response.id, "timeout-sync-1");
    assert!(response.result.is_none());
    assert_eq!(
        response.error_code.as_deref(),
        Some(IPC_HANDLER_TIMEOUT_CODE)
    );
    assert!(
        response
            .error
            .as_deref()
            .is_some_and(|message| message.contains("timed out after 5ms"))
    );
}

#[test]
fn queued_ipc_requests_consume_the_same_end_to_end_timeout_budget() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    ipcMain.handle('busy-sync', () => {
                        const startedAt = Date.now();
                        while ((Date.now() - startedAt) < 50) {
                        }
                        return 'done';
                    });
                    ipcMain.handle('fast', () => 'ok');
                    return 'registered';
                })()",
        )
        .expect("register busy and fast handlers");

    let busy_timeout = Duration::from_millis(200);
    let (busy_response_tx, busy_response_rx) = mpsc::channel();
    client
        .request_tx
        .send(RuntimeRequest::DispatchIpc {
            raw: r#"{"id":"busy-1","method":"busy-sync","args":null}"#.to_string(),
            timeout: busy_timeout,
            deadline: Instant::now() + busy_timeout,
            response_tx: busy_response_tx,
        })
        .expect("queue busy request");

    let response = client
        .dispatch_ipc_message(
            r#"{"id":"queued-timeout-1","method":"fast","args":null}"#,
            Duration::from_millis(20),
        )
        .expect("queued request response");

    assert_eq!(response.id, "queued-timeout-1");
    assert!(response.result.is_none());
    assert_eq!(
        response.error_code.as_deref(),
        Some(IPC_HANDLER_TIMEOUT_CODE)
    );
    assert!(
        response
            .error_details
            .as_ref()
            .and_then(|details| details.get("queueDelayMs"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default()
            > 0
    );

    let busy_response = busy_response_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("busy response");
    assert_eq!(busy_response.result, Some(serde_json::json!("done")));
}

#[test]
fn ipc_roundtrip_rejects_payload_over_limit() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();
    let oversized = format!(
        r#"{{"id":"big-1","method":"echo","args":"{}"}}"#,
        "x".repeat(IPC_MAX_REQUEST_BYTES + 1)
    );

    let response = dispatch_ipc_request(&client, &oversized);
    assert_eq!(response.id, "big-1");
    assert_eq!(
        response.error_code.as_deref(),
        Some("IPC_PAYLOAD_TOO_LARGE")
    );
    assert!(response.result.is_none());
}

#[test]
fn ipc_roundtrip_rejects_prototype_pollution_payloads() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let response = dispatch_ipc_request(
        &runtime.client(),
        r#"{"id":"pollute-1","method":"test","args":{"__proto__":{"polluted":true}}}"#,
    );

    assert_eq!(response.id, "pollute-1");
    assert!(
        response
            .error
            .as_deref()
            .is_some_and(|message| message.contains("prototype pollution"))
    );
    assert_eq!(response.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
}
