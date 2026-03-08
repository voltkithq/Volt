use std::collections::HashMap;

use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;

use crate::command;

pub(super) fn ensure_shutdown_cleanup(
    shutdown_cleanup_done: &mut bool,
    bridge_lifecycle: &command::BridgeLifecycle,
    hotkey_manager: &GlobalHotKeyManager,
    registered_hotkeys: &mut HashMap<String, HotKey>,
    app_menu: &mut Option<muda::Menu>,
    tray_handle: &mut Option<crate::tray::TrayHandle>,
) {
    if !super::begin_shutdown_cleanup(shutdown_cleanup_done) {
        return;
    }

    let mut first_error: Option<String> = None;
    for (accelerator, hotkey) in std::mem::take(registered_hotkeys) {
        if let Err(err) = hotkey_manager.unregister(hotkey)
            && first_error.is_none()
        {
            first_error = Some(format!("Failed to unregister '{accelerator}': {err}"));
        }
    }

    if let Some(message) = first_error {
        log::warn!("{message}");
    }

    app_menu.take();
    tray_handle.take();
    bridge_lifecycle.shutdown();
}

pub(super) fn begin_shutdown_cleanup(shutdown_cleanup_done: &mut bool) -> bool {
    if *shutdown_cleanup_done {
        return false;
    }
    *shutdown_cleanup_done = true;
    true
}
