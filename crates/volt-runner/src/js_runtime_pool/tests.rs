use std::time::Duration;

use serde_json::Value as JsonValue;
use volt_core::ipc::IPC_HANDLER_ERROR_CODE;

use super::*;

#[test]
fn normalize_pool_size_clamps_to_supported_bounds() {
    assert_eq!(normalize_pool_size(0), MIN_POOL_SIZE);
    assert_eq!(normalize_pool_size(1), MIN_POOL_SIZE);
    assert_eq!(normalize_pool_size(2), 2);
    assert_eq!(normalize_pool_size(4), 4);
    assert_eq!(normalize_pool_size(16), MAX_POOL_SIZE);
}

#[test]
fn pool_starts_requested_runtime_count_with_bounds() {
    let pool = JsRuntimePool::start_with_options(1, JsRuntimeOptions::default())
        .expect("runtime pool start");
    assert_eq!(pool.runtime_count(), MIN_POOL_SIZE);

    let pool = JsRuntimePool::start_with_options(8, JsRuntimeOptions::default())
        .expect("runtime pool start");
    assert_eq!(pool.runtime_count(), MAX_POOL_SIZE);
}

#[test]
fn ipc_dispatch_uses_single_stateful_runtime() {
    let pool = JsRuntimePool::start_with_options(2, JsRuntimeOptions::default())
        .expect("runtime pool start");
    let client = pool.client();
    client
        .load_backend_bundle(
            r#"
                let counter = 0;
                const { ipcMain } = await import('volt:ipc');
                ipcMain.handle('runtime-counter', () => {
                    counter += 1;
                    return counter;
                });
            "#,
        )
        .expect("load backend bundle");

    for index in 0..8 {
        let request = format!(r#"{{"id":"req-{index}","method":"runtime-counter","args":null}}"#);
        let response = client
            .dispatch_ipc_message(&request, Duration::from_secs(2))
            .expect("dispatch request");
        assert!(
            response.error.is_none(),
            "unexpected response error: {:?}",
            response.error
        );
        let value = response
            .result
            .as_ref()
            .and_then(JsonValue::as_i64)
            .expect("counter value");
        assert_eq!(value, index + 1);
    }
}

#[test]
fn native_events_are_dispatched_to_the_stateful_runtime() {
    let pool = JsRuntimePool::start_with_options(2, JsRuntimeOptions::default())
        .expect("runtime pool start");
    let client = pool.client();
    client
        .load_backend_bundle(
            r#"
                let hits = 0;
                globalThis.__volt_native_event_on__('menu:click', () => {
                    hits += 1;
                });
                const { ipcMain } = await import('volt:ipc');
                ipcMain.handle('native-event-hits', () => hits);
            "#,
        )
        .expect("load backend bundle");

    client
        .dispatch_native_event("menu:click", serde_json::json!({ "menuId": "demo" }))
        .expect("dispatch native event");

    let response_a = client
        .dispatch_ipc_message(
            r#"{"id":"hits-a","method":"native-event-hits","args":null}"#,
            Duration::from_secs(2),
        )
        .expect("dispatch hits request a");
    let response_b = client
        .dispatch_ipc_message(
            r#"{"id":"hits-b","method":"native-event-hits","args":null}"#,
            Duration::from_secs(2),
        )
        .expect("dispatch hits request b");

    let first = response_a
        .result
        .as_ref()
        .and_then(JsonValue::as_i64)
        .expect("first hit counter");
    let second = response_b
        .result
        .as_ref()
        .and_then(JsonValue::as_i64)
        .expect("second hit counter");

    assert_eq!(first, 1);
    assert_eq!(second, 1);
}

#[test]
fn eval_i64_uses_single_stateful_runtime() {
    let pool = JsRuntimePool::start_with_options(2, JsRuntimeOptions::default())
        .expect("runtime pool start");
    let client = pool.client();
    client
        .load_backend_bundle(
            r#"
                let evalHits = 0;
                globalThis.__volt_mark_eval_hit__ = () => {
                    evalHits += 1;
                    return evalHits;
                };
            "#,
        )
        .expect("load backend bundle");

    let first = client
        .eval_i64("globalThis.__volt_mark_eval_hit__()")
        .expect("first eval");
    let second = client
        .eval_i64("globalThis.__volt_mark_eval_hit__()")
        .expect("second eval");

    assert_eq!(first, 1);
    assert_eq!(second, 2);
}

#[test]
fn ipc_rate_limit_is_shared_across_pool_runtimes() {
    let pool = JsRuntimePool::start_with_options(2, JsRuntimeOptions::default())
        .expect("runtime pool start");
    let client = pool.client();

    {
        let mut limiter = client
            .ipc_rate_limiter
            .lock()
            .expect("acquire shared IPC rate limiter");
        limiter.fill_to_limit();
    }

    let response = client
        .dispatch_ipc_message(
            r#"{"id":"rate-limit-1","method":"any","args":null}"#,
            Duration::from_secs(2),
        )
        .expect("dispatch response");
    assert_eq!(response.id, "rate-limit-1");
    assert_eq!(response.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
    assert!(
        response
            .error
            .as_deref()
            .is_some_and(|message| message.contains("rate limit exceeded"))
    );
}
