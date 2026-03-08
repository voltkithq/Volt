use std::collections::HashMap;
use tao::event_loop::ControlFlow;

use crate::command::AppCommand;

use super::super::AppEvent;
use super::super::window_management::{
    debug_assert_window_invariants, mark_all_windows_closed, remove_window_and_maybe_quit,
};
use super::menu::apply_menu_to_windows;
use super::{CommandContext, parse_hotkey_accelerator};

pub(super) fn handle_command<F>(cmd: AppCommand, context: &mut CommandContext<'_, F>) -> bool
where
    F: FnMut(&AppEvent),
{
    let windows = &mut *context.windows;
    let window_states = &mut *context.window_states;
    let js_to_tao = &mut *context.js_to_tao;
    let tao_to_js = &mut *context.tao_to_js;
    let hotkey_manager = context.hotkey_manager;
    let registered_hotkeys = &mut *context.registered_hotkeys;
    let app_menu = &mut *context.app_menu;
    let tray_handle = &mut *context.tray_handle;
    let control_flow = &mut *context.control_flow;
    let on_event = &mut *context.on_event;

    match cmd {
        AppCommand::CloseWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id).copied() {
                return remove_window_and_maybe_quit(
                    window_id,
                    windows,
                    window_states,
                    js_to_tao,
                    tao_to_js,
                    control_flow,
                    on_event,
                );
            }
        }
        AppCommand::ShowWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((window, _webview)) = windows.get(window_id)
            {
                window.set_visible(true);
            }
        }
        AppCommand::FocusWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((window, _webview)) = windows.get(window_id)
            {
                window.focus();
            }
        }
        AppCommand::MaximizeWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((window, _webview)) = windows.get(window_id)
            {
                window.maximize();
            }
        }
        AppCommand::MinimizeWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((window, _webview)) = windows.get(window_id)
            {
                window.minimize();
            }
        }
        AppCommand::RestoreWindow { js_id } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((window, _webview)) = windows.get(window_id)
            {
                window.restore();
            }
        }
        AppCommand::EvaluateScript { js_id, script } => {
            if let Some(window_id) = js_to_tao.get(&js_id)
                && let Some((_window, webview)) = windows.get(window_id)
                && let Err(e) = webview.evaluate_script(&script)
            {
                log::error!("Failed to evaluate script in {js_id}: {e}");
            }
        }
        AppCommand::EmitEvent {
            js_window_id,
            event_name,
            data,
        } => {
            let script = match crate::ipc::event_script(&event_name, &data) {
                Ok(script) => script,
                Err(error) => {
                    log::error!("Failed to generate event script for '{event_name}': {error}");
                    return false;
                }
            };

            match js_window_id {
                Some(js_id) => {
                    if let Some(window_id) = js_to_tao.get(&js_id)
                        && let Some((_window, webview)) = windows.get(window_id)
                        && let Err(error) = webview.evaluate_script(&script)
                    {
                        log::error!("Failed to emit event '{event_name}' to {js_id}: {error}");
                    }
                }
                None => {
                    for (window_id, (_window, webview)) in windows.iter() {
                        if let Err(error) = webview.evaluate_script(&script) {
                            log::error!(
                                "Failed to emit event '{event_name}' to window {window_id:?}: {error}"
                            );
                        }
                    }
                }
            }
        }
        AppCommand::GetWindowCount { reply } => {
            let _ = reply.send(windows.len() as u32);
        }
        AppCommand::IpcMessage { js_window_id, raw } => {
            on_event(&AppEvent::IpcMessage { js_window_id, raw });
        }
        AppCommand::SetAppMenu { items, reply } => {
            let result = apply_menu_to_windows(windows, &items, app_menu);
            let _ = reply.send(result);
        }
        AppCommand::RegisterShortcut { accelerator, reply } => {
            let result = parse_hotkey_accelerator(&accelerator).and_then(|hotkey| {
                hotkey_manager
                    .register(hotkey)
                    .map_err(|e| format!("Failed to register '{accelerator}': {e}"))?;
                let id = hotkey.id();
                registered_hotkeys.insert(accelerator, hotkey);
                Ok(id)
            });
            let _ = reply.send(result);
        }
        AppCommand::UnregisterShortcut { accelerator, reply } => {
            let result = if let Some(hotkey) = registered_hotkeys.remove(&accelerator) {
                hotkey_manager
                    .unregister(hotkey)
                    .map_err(|e| format!("Failed to unregister '{accelerator}': {e}"))
            } else {
                Ok(())
            };
            let _ = reply.send(result);
        }
        AppCommand::UnregisterAllShortcuts { reply } => {
            let result = unregister_all_shortcuts(registered_hotkeys, |accelerator, hotkey| {
                hotkey_manager
                    .unregister(*hotkey)
                    .map_err(|err| format!("Failed to unregister '{accelerator}': {err}"))
            });
            let _ = reply.send(result);
        }
        AppCommand::CreateTray { config, reply } => {
            let previous_tray = tray_handle.take();

            let result = crate::tray::TrayHandle::new(&crate::tray::TrayConfig {
                tooltip: config.tooltip,
                icon: config.icon_rgba,
                icon_width: config.icon_width,
                icon_height: config.icon_height,
            })
            .map(|handle| {
                let tray_id = handle.id().to_string();
                *tray_handle = Some(handle);
                tray_id
            })
            .map_err(|error| format!("Failed to create tray: {error}"));

            if result.is_err() {
                *tray_handle = previous_tray;
            }

            let _ = reply.send(result);
        }
        AppCommand::SetTrayTooltip { tooltip, reply } => {
            let result = tray_handle
                .as_ref()
                .ok_or_else(|| "Tray has not been created".to_string())
                .and_then(|handle| {
                    handle
                        .set_tooltip(&tooltip)
                        .map_err(|error| format!("Failed to set tray tooltip: {error}"))
                });
            let _ = reply.send(result);
        }
        AppCommand::SetTrayVisible { visible, reply } => {
            let result = tray_handle
                .as_ref()
                .ok_or_else(|| "Tray has not been created".to_string())
                .and_then(|handle| {
                    handle
                        .set_visible(visible)
                        .map_err(|error| format!("Failed to set tray visibility: {error}"))
                });
            let _ = reply.send(result);
        }
        AppCommand::DestroyTray { reply } => {
            tray_handle.take();
            let _ = reply.send(Ok(()));
        }
        AppCommand::Quit => {
            mark_all_windows_closed(window_states, windows.keys().copied());
            windows.clear();
            js_to_tao.clear();
            tao_to_js.clear();
            debug_assert_window_invariants(windows, js_to_tao, tao_to_js, window_states);
            *control_flow = ControlFlow::Exit;
            on_event(&AppEvent::Quit);
            return true;
        }
    }

    false
}

fn unregister_all_shortcuts<T>(
    registered_shortcuts: &mut HashMap<String, T>,
    mut unregister: impl FnMut(&str, &T) -> Result<(), String>,
) -> Result<(), String> {
    let mut first_error: Option<String> = None;
    let mut failed_shortcuts = HashMap::new();

    for (accelerator, hotkey) in std::mem::take(registered_shortcuts) {
        if let Err(error) = unregister(&accelerator, &hotkey) {
            if first_error.is_none() {
                first_error = Some(error);
            }
            failed_shortcuts.insert(accelerator, hotkey);
        }
    }

    *registered_shortcuts = failed_shortcuts;
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::unregister_all_shortcuts;
    use std::collections::HashMap;

    #[test]
    fn test_unregister_all_shortcuts_continues_after_failure_and_preserves_failed_entries() {
        let mut shortcuts = HashMap::new();
        shortcuts.insert("a".to_string(), 1_u8);
        shortcuts.insert("b".to_string(), 2_u8);
        shortcuts.insert("c".to_string(), 3_u8);

        let mut attempts = Vec::new();
        let result = unregister_all_shortcuts(&mut shortcuts, |accelerator, _| {
            attempts.push(accelerator.to_string());
            if accelerator == "b" {
                Err("failed b".to_string())
            } else {
                Ok(())
            }
        });

        assert!(result.is_err());
        assert_eq!(attempts.len(), 3);
        assert_eq!(shortcuts.len(), 1);
        assert!(shortcuts.contains_key("b"));
    }

    #[test]
    fn test_unregister_all_shortcuts_clears_all_on_success() {
        let mut shortcuts = HashMap::new();
        shortcuts.insert("a".to_string(), 1_u8);
        shortcuts.insert("b".to_string(), 2_u8);

        let result = unregister_all_shortcuts(&mut shortcuts, |_accelerator, _| Ok(()));

        assert!(result.is_ok());
        assert!(shortcuts.is_empty());
    }
}
