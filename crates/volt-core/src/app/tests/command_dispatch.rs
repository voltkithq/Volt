use super::try_hotkey_manager;
use crate::app;
use crate::command::AppCommand;
use std::collections::HashMap;
use std::sync::mpsc;
use tao::event_loop::ControlFlow;

#[test]
fn test_handle_command_get_window_count_reports_store_size() {
    let Some(hotkey_manager) = try_hotkey_manager() else {
        return;
    };

    let mut windows = app::WindowStore::new();
    let mut window_states = app::WindowStateStore::new();
    let mut js_to_tao = HashMap::new();
    let mut tao_to_js = HashMap::new();
    let mut registered_hotkeys = HashMap::new();
    let mut app_menu = None;
    let mut tray_handle = None;
    let mut control_flow = ControlFlow::Wait;
    let mut observed = Vec::<app::AppEvent>::new();
    let mut on_event = |event: &app::AppEvent| observed.push(event.clone());

    let (reply_tx, reply_rx) = mpsc::channel();
    let should_shutdown = {
        let mut context = app::command_handling::CommandContext {
            windows: &mut windows,
            window_states: &mut window_states,
            js_to_tao: &mut js_to_tao,
            tao_to_js: &mut tao_to_js,
            hotkey_manager: &hotkey_manager,
            registered_hotkeys: &mut registered_hotkeys,
            app_menu: &mut app_menu,
            tray_handle: &mut tray_handle,
            control_flow: &mut control_flow,
            on_event: &mut on_event,
        };
        app::command_handling::handle_command(
            AppCommand::GetWindowCount { reply: reply_tx },
            &mut context,
        )
    };

    assert!(!should_shutdown);
    assert_eq!(reply_rx.recv().expect("window count reply"), 0);
    assert!(observed.is_empty());
}

#[test]
fn test_handle_command_ipc_message_forwards_to_event_callback() {
    let Some(hotkey_manager) = try_hotkey_manager() else {
        return;
    };

    let mut windows = app::WindowStore::new();
    let mut window_states = app::WindowStateStore::new();
    let mut js_to_tao = HashMap::new();
    let mut tao_to_js = HashMap::new();
    let mut registered_hotkeys = HashMap::new();
    let mut app_menu = None;
    let mut tray_handle = None;
    let mut control_flow = ControlFlow::Wait;
    let mut observed = Vec::<app::AppEvent>::new();
    let mut on_event = |event: &app::AppEvent| observed.push(event.clone());

    let should_shutdown = {
        let mut context = app::command_handling::CommandContext {
            windows: &mut windows,
            window_states: &mut window_states,
            js_to_tao: &mut js_to_tao,
            tao_to_js: &mut tao_to_js,
            hotkey_manager: &hotkey_manager,
            registered_hotkeys: &mut registered_hotkeys,
            app_menu: &mut app_menu,
            tray_handle: &mut tray_handle,
            control_flow: &mut control_flow,
            on_event: &mut on_event,
        };
        app::command_handling::handle_command(
            AppCommand::IpcMessage {
                js_window_id: "window-1".to_string(),
                raw: "{\"id\":1}".to_string(),
            },
            &mut context,
        )
    };

    assert!(!should_shutdown);
    assert_eq!(observed.len(), 1);
    match &observed[0] {
        app::AppEvent::IpcMessage { js_window_id, raw } => {
            assert_eq!(js_window_id, "window-1");
            assert_eq!(raw, "{\"id\":1}");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn test_handle_command_set_tray_tooltip_replies_error_without_tray() {
    let Some(hotkey_manager) = try_hotkey_manager() else {
        return;
    };

    let mut windows = app::WindowStore::new();
    let mut window_states = app::WindowStateStore::new();
    let mut js_to_tao = HashMap::new();
    let mut tao_to_js = HashMap::new();
    let mut registered_hotkeys = HashMap::new();
    let mut app_menu = None;
    let mut tray_handle = None;
    let mut control_flow = ControlFlow::Wait;
    let mut observed = Vec::<app::AppEvent>::new();
    let mut on_event = |event: &app::AppEvent| observed.push(event.clone());

    let (reply_tx, reply_rx) = mpsc::channel();
    let should_shutdown = {
        let mut context = app::command_handling::CommandContext {
            windows: &mut windows,
            window_states: &mut window_states,
            js_to_tao: &mut js_to_tao,
            tao_to_js: &mut tao_to_js,
            hotkey_manager: &hotkey_manager,
            registered_hotkeys: &mut registered_hotkeys,
            app_menu: &mut app_menu,
            tray_handle: &mut tray_handle,
            control_flow: &mut control_flow,
            on_event: &mut on_event,
        };
        app::command_handling::handle_command(
            AppCommand::SetTrayTooltip {
                tooltip: "Volt".to_string(),
                reply: reply_tx,
            },
            &mut context,
        )
    };

    assert!(!should_shutdown);
    assert!(reply_rx.recv().expect("tray tooltip reply").is_err());
    assert!(observed.is_empty());
}

#[test]
fn test_handle_command_quit_clears_state_and_requests_shutdown() {
    let Some(hotkey_manager) = try_hotkey_manager() else {
        return;
    };

    let mut windows = app::WindowStore::new();
    let mut window_states = app::WindowStateStore::new();
    let mut js_to_tao = HashMap::new();
    let mut tao_to_js = HashMap::new();
    let mut registered_hotkeys = HashMap::new();
    let mut app_menu = None;
    let mut tray_handle = None;
    let mut control_flow = ControlFlow::Wait;
    let mut observed = Vec::<app::AppEvent>::new();
    let mut on_event = |event: &app::AppEvent| observed.push(event.clone());

    let should_shutdown = {
        let mut context = app::command_handling::CommandContext {
            windows: &mut windows,
            window_states: &mut window_states,
            js_to_tao: &mut js_to_tao,
            tao_to_js: &mut tao_to_js,
            hotkey_manager: &hotkey_manager,
            registered_hotkeys: &mut registered_hotkeys,
            app_menu: &mut app_menu,
            tray_handle: &mut tray_handle,
            control_flow: &mut control_flow,
            on_event: &mut on_event,
        };
        app::command_handling::handle_command(AppCommand::Quit, &mut context)
    };

    assert!(should_shutdown);
    assert!(matches!(control_flow, ControlFlow::Exit));
    assert!(windows.is_empty());
    assert!(js_to_tao.is_empty());
    assert!(tao_to_js.is_empty());
    assert!(
        observed
            .iter()
            .any(|event| matches!(event, app::AppEvent::Quit))
    );
}
