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
    let _ = std::fs::remove_dir_all(fs_base_dir);
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
    let _ = std::fs::remove_dir_all(fs_base_dir);
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
    let _ = std::fs::remove_dir_all(fs_base_dir);
}
