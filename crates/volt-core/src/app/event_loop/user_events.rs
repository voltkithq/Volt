use super::super::command_handling::ensure_shutdown_cleanup;
use super::super::window_management::{
    mark_all_windows_closed, remove_window_and_maybe_quit, set_window_active,
};
use super::super::{AppConfig, AppEvent, WindowStateStore, WindowStore, allocate_js_window_id};
use crate::command;
use crate::embed::AssetBundle;
use crate::webview::create_webview;
use crate::window::create_window;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use std::collections::HashMap;
use std::sync::Arc;
use tao::event_loop::{ControlFlow, EventLoopWindowTarget};
use wry::WebContext;

pub(super) struct UserEventContext<'a, F>
where
    F: FnMut(&AppEvent),
{
    pub(super) windows: &'a mut WindowStore,
    pub(super) window_states: &'a mut WindowStateStore,
    pub(super) js_to_tao: &'a mut HashMap<String, tao::window::WindowId>,
    pub(super) tao_to_js: &'a mut HashMap<tao::window::WindowId, String>,
    pub(super) bridge_lifecycle: &'a command::BridgeLifecycle,
    pub(super) hotkey_manager: &'a GlobalHotKeyManager,
    pub(super) registered_hotkeys: &'a mut HashMap<String, HotKey>,
    pub(super) app_menu: &'a mut Option<muda::Menu>,
    pub(super) tray_handle: &'a mut Option<crate::tray::TrayHandle>,
    pub(super) shutdown_cleanup_done: &'a mut bool,
    pub(super) control_flow: &'a mut ControlFlow,
    pub(super) on_event: &'a mut F,
    pub(super) config: &'a AppConfig,
    pub(super) asset_bundle: Option<Arc<AssetBundle>>,
    pub(super) web_context: &'a mut WebContext,
}

pub(super) fn handle_user_event<F>(
    app_event: &AppEvent,
    event_loop_window_target: &EventLoopWindowTarget<AppEvent>,
    context: &mut UserEventContext<'_, F>,
) where
    F: FnMut(&AppEvent),
{
    match app_event {
        AppEvent::CreateWindow {
            window_config,
            webview_config,
            js_window_id,
        } => create_window_for_event(
            event_loop_window_target,
            window_config,
            webview_config,
            js_window_id,
            context,
        ),
        AppEvent::CloseWindow(window_id) => {
            let should_shutdown = remove_window_and_maybe_quit(
                *window_id,
                context.windows,
                context.window_states,
                context.js_to_tao,
                context.tao_to_js,
                context.control_flow,
                context.on_event,
            );
            if should_shutdown {
                shutdown_now(context);
            }
        }
        AppEvent::Quit => {
            mark_all_windows_closed(context.window_states, context.windows.keys().copied());
            context.windows.clear();
            context.js_to_tao.clear();
            context.tao_to_js.clear();
            *context.control_flow = ControlFlow::Exit;
            shutdown_now(context);
        }
        AppEvent::EvaluateScript { window_id, script } => {
            if let Some((_handle, webview)) = context.windows.get(window_id)
                && let Err(error) = webview.evaluate_script(script)
            {
                log::error!("Failed to evaluate script: {error}");
            }
        }
        AppEvent::ProcessCommands
        | AppEvent::IpcMessage { .. }
        | AppEvent::MenuEvent { .. }
        | AppEvent::ShortcutTriggered { .. }
        | AppEvent::TrayEvent { .. } => {}
    }
}

fn create_window_for_event<F>(
    event_loop_window_target: &EventLoopWindowTarget<AppEvent>,
    window_config: &crate::window::WindowConfig,
    webview_config: &crate::webview::WebViewConfig,
    js_window_id: &Option<String>,
    context: &mut UserEventContext<'_, F>,
) where
    F: FnMut(&AppEvent),
{
    let resolved_js_id = js_window_id.clone().unwrap_or_else(allocate_js_window_id);
    match create_window(event_loop_window_target, window_config) {
        Ok(window_handle) => {
            match create_webview(
                window_handle.inner(),
                webview_config,
                context.config.devtools,
                context.asset_bundle.clone(),
                resolved_js_id.clone(),
                context.web_context,
            ) {
                Ok(webview) => {
                    let id = window_handle.id();
                    context.windows.insert(id, (window_handle, webview));
                    set_window_active(context.window_states, id);
                    context.js_to_tao.insert(resolved_js_id.clone(), id);
                    context.tao_to_js.insert(id, resolved_js_id);
                }
                Err(error) => {
                    log::error!("Failed to create webview: {error}");
                }
            }
        }
        Err(error) => {
            log::error!("Failed to create window: {error}");
        }
    }
}

fn shutdown_now<F>(context: &mut UserEventContext<'_, F>)
where
    F: FnMut(&AppEvent),
{
    ensure_shutdown_cleanup(
        context.shutdown_cleanup_done,
        context.bridge_lifecycle,
        context.hotkey_manager,
        context.registered_hotkeys,
        context.app_menu,
        context.tray_handle,
    );
}
