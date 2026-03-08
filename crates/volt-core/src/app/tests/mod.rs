use global_hotkey::GlobalHotKeyManager;

mod command_dispatch;
mod config;
mod event_types;
mod window_management;

fn try_hotkey_manager() -> Option<GlobalHotKeyManager> {
    GlobalHotKeyManager::new().ok()
}
