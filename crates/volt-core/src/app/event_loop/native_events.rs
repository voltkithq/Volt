use std::collections::HashMap;

use super::super::AppEvent;

pub(super) fn poll_native_runtime_events<F>(
    on_event: &mut F,
    menu_id_map: &HashMap<String, String>,
) where
    F: FnMut(&AppEvent),
{
    if let Some(menu_event) = crate::menu::check_menu_event() {
        let internal_menu_id = menu_event.id().0.to_string();
        let resolved_menu_id = menu_id_map
            .get(&internal_menu_id)
            .cloned()
            .unwrap_or(internal_menu_id);
        on_event(&AppEvent::MenuEvent {
            menu_id: resolved_menu_id,
        });
    }

    if let Ok(shortcut_event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
        on_event(&AppEvent::ShortcutTriggered {
            id: shortcut_event.id(),
        });
    }

    while let Ok(tray_event) = tray_icon::TrayIconEvent::receiver().try_recv() {
        if matches!(tray_event, tray_icon::TrayIconEvent::Click { .. }) {
            on_event(&AppEvent::TrayEvent {
                tray_id: tray_event.id().as_ref().to_string(),
            });
        }
    }
}
