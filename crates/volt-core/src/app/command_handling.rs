use super::{AppEvent, WindowStateStore, WindowStore};
use crate::command::{self, AppCommand};
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use std::collections::HashMap;
use std::time::Duration;
use tao::event_loop::ControlFlow;

mod cleanup;
mod dispatch;
mod hotkeys;
mod menu;
mod obs;

pub(super) struct CommandContext<'a, F>
where
    F: FnMut(&AppEvent),
{
    pub(super) windows: &'a mut WindowStore,
    pub(super) window_states: &'a mut WindowStateStore,
    pub(super) js_to_tao: &'a mut HashMap<String, tao::window::WindowId>,
    pub(super) tao_to_js: &'a mut HashMap<tao::window::WindowId, String>,
    pub(super) hotkey_manager: &'a GlobalHotKeyManager,
    pub(super) registered_hotkeys: &'a mut HashMap<String, HotKey>,
    pub(super) app_menu: &'a mut Option<muda::Menu>,
    pub(super) tray_handle: &'a mut Option<crate::tray::TrayHandle>,
    pub(super) control_flow: &'a mut ControlFlow,
    pub(super) on_event: &'a mut F,
}

pub(super) fn handle_command<F>(cmd: AppCommand, context: &mut CommandContext<'_, F>) -> bool
where
    F: FnMut(&AppEvent),
{
    dispatch::handle_command(cmd, context)
}

pub(super) fn log_command_observability(
    trace_id: u64,
    queue_delay: Duration,
    processing_duration: Duration,
) {
    obs::log_command_observability(trace_id, queue_delay, processing_duration);
}

pub(super) fn ensure_shutdown_cleanup(
    shutdown_cleanup_done: &mut bool,
    bridge_lifecycle: &command::BridgeLifecycle,
    hotkey_manager: &GlobalHotKeyManager,
    registered_hotkeys: &mut HashMap<String, HotKey>,
    app_menu: &mut Option<muda::Menu>,
    tray_handle: &mut Option<crate::tray::TrayHandle>,
) {
    cleanup::ensure_shutdown_cleanup(
        shutdown_cleanup_done,
        bridge_lifecycle,
        hotkey_manager,
        registered_hotkeys,
        app_menu,
        tray_handle,
    );
}

pub(super) fn begin_shutdown_cleanup(shutdown_cleanup_done: &mut bool) -> bool {
    cleanup::begin_shutdown_cleanup(shutdown_cleanup_done)
}

pub(super) fn parse_hotkey_accelerator(accelerator: &str) -> Result<HotKey, String> {
    hotkeys::parse_hotkey_accelerator(accelerator)
}
