use super::*;

#[test]
fn native_event_dispatch_reaches_menu_handlers() {
    let fs_base_dir = unique_temp_dir("native-menu-events");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["menu"]);
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const menu = globalThis.__volt.menu;
                    let captured = '';
                    menu.on('click', (payload) => {
                        captured = payload && typeof payload.menuId === 'string' ? payload.menuId : '';
                    });
                    globalThis.__menu_capture__ = () => captured;
                    return 'ready';
                })()",
        )
        .expect("register menu handler");

    dispatch_native_event(
        &client,
        "menu:click",
        serde_json::json!({ "menuId": "file-open" }),
    );
    let captured = client
        .eval_string("globalThis.__menu_capture__()")
        .expect("menu capture");
    assert_eq!(captured, "file-open");
    let _ = fs::remove_dir_all(fs_base_dir);
}

#[test]
fn native_event_off_unregisters_shortcut_handler() {
    let fs_base_dir = unique_temp_dir("native-shortcut-events");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["globalShortcut"]);
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const globalShortcut = globalThis.__volt.globalShortcut;
                    globalThis.__shortcut_hits__ = 0;
                    const handler = () => {
                        globalThis.__shortcut_hits__ += 1;
                    };
                    globalShortcut.on('triggered', handler);
                    globalThis.__shortcut_handler__ = handler;
                    return 'ready';
                })()",
        )
        .expect("register shortcut handler");

    dispatch_native_event(
        &client,
        "shortcut:triggered",
        serde_json::json!({ "id": 1 }),
    );
    client
        .eval_unit(
            "globalThis.__volt.globalShortcut.off('triggered', globalThis.__shortcut_handler__)",
        )
        .expect("unregister shortcut handler");
    dispatch_native_event(
        &client,
        "shortcut:triggered",
        serde_json::json!({ "id": 1 }),
    );

    let hits = client
        .eval_i64("globalThis.__shortcut_hits__")
        .expect("shortcut hits");
    assert_eq!(hits, 1);
    let _ = fs::remove_dir_all(fs_base_dir);
}

#[test]
fn native_event_dispatch_reaches_tray_handlers() {
    let fs_base_dir = unique_temp_dir("native-tray-events");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["tray"]);
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const tray = globalThis.__volt.tray;
                    let captured = '';
                    tray.on('click', (payload) => {
                        captured = payload && typeof payload.trayId === 'string' ? payload.trayId : '';
                    });
                    globalThis.__tray_capture__ = () => captured;
                    return 'ready';
                })()",
        )
        .expect("register tray handler");

    dispatch_native_event(
        &client,
        "tray:click",
        serde_json::json!({ "trayId": "tray-1" }),
    );
    let captured = client
        .eval_string("globalThis.__tray_capture__()")
        .expect("tray capture");
    assert_eq!(captured, "tray-1");
    let _ = fs::remove_dir_all(fs_base_dir);
}

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

#[test]
fn ipc_handler_can_use_native_modules() {
    let fs_base_dir = unique_temp_dir("ipc-fs-handler");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["fs"]);
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    const fs = globalThis.__volt.fs;
                    ipcMain.handle('save-document', async (args) => {
                        await fs.writeFile(args.path, args.content);
                        const loaded = await fs.readFile(args.path);
                        await fs.remove(args.path);
                        return { saved: true, loaded };
                    });
                    return 'registered';
                })()",
        )
        .expect("register fs ipc handler");

    let response = dispatch_ipc_request(
        &client,
        r#"{"id":"fs-1","method":"save-document","args":{"path":"doc.txt","content":"hello"}}"#,
    );
    assert_eq!(response.id, "fs-1");
    assert_eq!(
        response.result,
        Some(serde_json::json!({ "saved": true, "loaded": "hello" }))
    );
    assert!(response.error.is_none());

    let _ = fs::remove_dir_all(fs_base_dir);
}
