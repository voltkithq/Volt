use super::*;

#[test]
fn evaluates_basic_js_expressions() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let result = runtime.client().eval_i64("1 + 41").expect("expression");
    assert_eq!(result, 42);
}

#[test]
fn console_log_is_available_and_callable() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    let console_log_exists = client
        .eval_bool("(typeof console === 'object') && (typeof console.log === 'function')")
        .expect("console.log should exist");
    assert!(console_log_exists);

    client
        .eval_unit("console.log('boa console test')")
        .expect("console.log call should succeed");
}

#[test]
fn timers_set_timeout_and_interval_work() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    let timeout_value = client
        .eval_promise_i64(
            "(async () => await new Promise((resolve) => { setTimeout(() => resolve(7), 5); }))()",
        )
        .expect("setTimeout promise");
    assert_eq!(timeout_value, 7);

    let interval_value = client
        .eval_promise_i64(
            "(async () => await new Promise((resolve) => { let count = 0; const handle = setInterval(() => { count += 1; if (count === 3) { clearInterval(handle); resolve(count); } }, 5); }))()",
        )
        .expect("setInterval promise");
    assert_eq!(interval_value, 3);
}

#[test]
fn async_await_promise_resolution_works() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let value = runtime
        .client()
        .eval_promise_i64(
            "(async () => { const value = await Promise.resolve(6); return value * 7; })()",
        )
        .expect("async await");
    assert_eq!(value, 42);
}

#[test]
fn rust_async_function_returns_promise_to_js() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let value = runtime
        .client()
        .eval_promise_string("(async () => await nativeSleep(1))()")
        .expect("native async call");
    assert_eq!(value, "slept:1");
}

#[test]
fn threaded_request_roundtrip_works() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    let worker = thread::spawn(move || client.eval_i64("40 + 2"));
    let result = worker.join().expect("thread join").expect("eval result");

    assert_eq!(result, 42);
}

#[test]
fn synchronous_string_evaluation_works() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let value = runtime
        .client()
        .eval_string("'volt'.toUpperCase()")
        .expect("string evaluation");
    assert_eq!(value, "VOLT");
}
