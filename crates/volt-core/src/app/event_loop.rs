use super::command_handling::{
    CommandContext, ensure_shutdown_cleanup, handle_command, log_command_observability,
};
use super::window_management::{debug_assert_window_invariants, remove_window_and_maybe_quit};
use super::{App, AppError, AppEvent};
use crate::command;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use std::collections::HashMap;
use std::time::Instant;
use tao::event::{ElementState, Event, WindowEvent};
use tao::event_loop::ControlFlow;
use tao::keyboard::KeyCode;

#[path = "event_loop/command_batch.rs"]
mod command_batch;
#[path = "event_loop/native_events.rs"]
mod native_events;
#[path = "event_loop/user_events.rs"]
mod user_events;

use command_batch::{MAX_COMMANDS_PER_TICK, drain_command_batch};
use user_events::{UserEventContext, handle_user_event};

pub(super) fn run_event_loop<F>(mut app: App, mut on_event: F) -> Result<(), AppError>
where
    F: FnMut(&AppEvent) + 'static,
{
    let event_loop = app.event_loop.take().ok_or(AppError::EventLoopConsumed)?;

    let bridge_registration =
        command::init_bridge(app.proxy.clone()).map_err(|e| AppError::Generic(e.to_string()))?;
    let cmd_rx = bridge_registration.receiver;
    let bridge_lifecycle = bridge_registration.lifecycle;

    let hotkey_manager = GlobalHotKeyManager::new()
        .map_err(|e| AppError::Generic(format!("Failed to create hotkey manager: {e}")))?;

    let mut windows = app.windows;
    let mut window_states = app.window_states;
    let mut js_to_tao = app.js_to_tao;
    let mut tao_to_js = app.tao_to_js;
    let mut registered_hotkeys: HashMap<String, HotKey> = HashMap::new();
    let mut app_menu: Option<muda::Menu> = None;
    let mut menu_id_map: HashMap<String, String> = HashMap::new();
    let mut tray_handle: Option<crate::tray::TrayHandle> = None;
    let config = app.config;
    let asset_bundle = app.asset_bundle;
    let mut web_context = app.web_context;
    let process_commands_proxy = app.proxy;

    debug_assert_window_invariants(&windows, &js_to_tao, &tao_to_js, &window_states);

    let mut shutdown_cleanup_done = false;
    event_loop.run(move |event, event_loop_window_target, control_flow| {
        *control_flow = ControlFlow::Wait;

        if !shutdown_cleanup_done {
            let (_processed, reached_batch_limit, should_shutdown) =
                drain_command_batch(&cmd_rx, MAX_COMMANDS_PER_TICK, |envelope| {
                let queue_delay = Instant::now().saturating_duration_since(envelope.enqueued_at);
                let command_started_at = Instant::now();
                let mut command_context = CommandContext {
                    windows: &mut windows,
                    window_states: &mut window_states,
                    js_to_tao: &mut js_to_tao,
                    tao_to_js: &mut tao_to_js,
                    hotkey_manager: &hotkey_manager,
                    registered_hotkeys: &mut registered_hotkeys,
                    app_menu: &mut app_menu,
                    menu_id_map: &mut menu_id_map,
                    tray_handle: &mut tray_handle,
                    control_flow,
                    on_event: &mut on_event,
                };
                let should_shutdown = handle_command(envelope.command, &mut command_context);
                let processing_duration = command_started_at.elapsed();
                command::record_processed_command();
                log_command_observability(envelope.trace_id, queue_delay, processing_duration);
                should_shutdown
            });

            if should_shutdown {
                ensure_shutdown_cleanup(
                    &mut shutdown_cleanup_done,
                    &bridge_lifecycle,
                    &hotkey_manager,
                    &mut registered_hotkeys,
                    &mut app_menu,
                    &mut tray_handle,
                );
            } else if reached_batch_limit {
                // Keep each loop tick bounded so menu/shortcut/tray events stay responsive.
                let _ = process_commands_proxy.send_event(AppEvent::ProcessCommands);
            }
        }

        native_events::poll_native_runtime_events(&mut on_event, &menu_id_map);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                let should_shutdown = remove_window_and_maybe_quit(
                    window_id,
                    &mut windows,
                    &mut window_states,
                    &mut js_to_tao,
                    &mut tao_to_js,
                    control_flow,
                    &mut on_event,
                );
                if should_shutdown {
                    ensure_shutdown_cleanup(
                        &mut shutdown_cleanup_done,
                        &bridge_lifecycle,
                        &hotkey_manager,
                        &mut registered_hotkeys,
                        &mut app_menu,
                        &mut tray_handle,
                    );
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                window_id,
                ..
            } => {
                let should_shutdown = remove_window_and_maybe_quit(
                    window_id,
                    &mut windows,
                    &mut window_states,
                    &mut js_to_tao,
                    &mut tao_to_js,
                    control_flow,
                    &mut on_event,
                );
                if should_shutdown {
                    ensure_shutdown_cleanup(
                        &mut shutdown_cleanup_done,
                        &bridge_lifecycle,
                        &hotkey_manager,
                        &mut registered_hotkeys,
                        &mut app_menu,
                        &mut tray_handle,
                    );
                }
            }

            // F12 toggles DevTools when devtools is enabled
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event: ref key_event,
                        ..
                    },
                window_id,
                ..
            } => {
                if config.devtools
                    && key_event.state == ElementState::Pressed
                    && !key_event.repeat
                    && key_event.physical_key == KeyCode::F12
                    && let Some((_handle, _webview)) = windows.get(&window_id)
                {
                    #[cfg(feature = "devtools")]
                    _webview.open_devtools();

                    #[cfg(not(feature = "devtools"))]
                    log::warn!(
                        "DevTools requested but support is not compiled in. Rebuild with `--features devtools`."
                    );
                }
            }

            Event::UserEvent(ref app_event) => {
                if !matches!(app_event, AppEvent::ProcessCommands) {
                    on_event(app_event);
                }

                let mut user_event_context = UserEventContext {
                    windows: &mut windows,
                    window_states: &mut window_states,
                    js_to_tao: &mut js_to_tao,
                    tao_to_js: &mut tao_to_js,
                    bridge_lifecycle: &bridge_lifecycle,
                    hotkey_manager: &hotkey_manager,
                    registered_hotkeys: &mut registered_hotkeys,
                    app_menu: &mut app_menu,
                    tray_handle: &mut tray_handle,
                    shutdown_cleanup_done: &mut shutdown_cleanup_done,
                    control_flow,
                    on_event: &mut on_event,
                    config: &config,
                    asset_bundle: asset_bundle.clone(),
                    web_context: &mut web_context,
                };

                handle_user_event(
                    app_event,
                    event_loop_window_target,
                    &mut user_event_context,
                );

                debug_assert_window_invariants(&windows, &js_to_tao, &tao_to_js, &window_states);
            }

            Event::LoopDestroyed => {
                ensure_shutdown_cleanup(
                    &mut shutdown_cleanup_done,
                    &bridge_lifecycle,
                    &hotkey_manager,
                    &mut registered_hotkeys,
                    &mut app_menu,
                    &mut tray_handle,
                );
                let snapshot = command::command_observability_snapshot();
                log::info!(
                    "Command observability summary: sent={} processed={} failed={}",
                    snapshot.commands_sent,
                    snapshot.commands_processed,
                    snapshot.commands_failed
                );
            }

            _ => {}
        }
    });
}
