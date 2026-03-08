use super::super::{JsRuntimeManager, runtime_with_permissions, unique_temp_dir};

#[test]
fn dialog_module_rejects_invalid_options_without_showing_ui() {
    let runtime = runtime_with_permissions(unique_temp_dir("dialog-module"), &["dialog"]);

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const dialog = globalThis.__volt.dialog;
                    let openInvalid = false;
                    let saveInvalid = false;
                    let messageInvalid = false;
                    try {
                        await dialog.showOpen({ multiple: 'invalid' });
                    } catch (_) { openInvalid = true; }
                    try {
                        await dialog.showSave({ filters: 'invalid' });
                    } catch (_) { saveInvalid = true; }
                    try {
                        await dialog.showMessage({ dialogType: 5 });
                    } catch (_) { messageInvalid = true; }
                    if (openInvalid && saveInvalid && messageInvalid) {
                        return 'invalid';
                    }
                    return 'unexpected';
                })()",
        )
        .expect("dialog validation script");
    assert_eq!(outcome, "invalid");
}

#[test]
fn clipboard_notification_and_shell_modules_enforce_permissions() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const clipboard = globalThis.__volt.clipboard;
                    const shell = globalThis.__volt.shell;
                    const notification = globalThis.__volt.notification;

                    let clipboardDenied = false;
                    let shellDenied = false;
                    let notificationDenied = false;

                    try { clipboard.readText(); } catch (error) {
                        clipboardDenied = String(error).includes('Permission denied');
                    }

                    try { await shell.openExternal('https://example.com'); } catch (error) {
                        shellDenied = String(error).includes('Permission denied');
                    }

                    try { notification.show({ title: 'hi' }); } catch (error) {
                        notificationDenied = String(error).includes('Permission denied');
                    }

                    return `${clipboardDenied}:${shellDenied}:${notificationDenied}`;
                })()",
        )
        .expect("permission guard script");

    assert_eq!(outcome, "true:true:true");
}

#[test]
fn tray_create_permission_checked_before_icon_read() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const tray = globalThis.__volt.tray;
                    try {
                        await tray.create({ icon: 'does-not-exist-icon.png' });
                        return 'unexpected';
                    } catch (error) {
                        return String(error).includes('Permission denied') ? 'denied' : String(error);
                    }
                })()",
        )
        .expect("tray permission script");

    assert_eq!(outcome, "denied");
}
