mod module_registry;
mod runtime_state;
mod runtime_values;

pub(crate) mod dialog_async;
mod event_helpers;
pub mod secure_storage;
#[cfg(test)]
pub mod test_utils;
pub mod volt_bench;
pub mod volt_clipboard;
pub mod volt_crypto;
pub mod volt_db;
pub mod volt_dialog;
pub mod volt_events;
pub mod volt_fs;
pub mod volt_global_shortcut;
pub mod volt_http;
pub mod volt_ipc;
pub mod volt_menu;
pub mod volt_notification;
pub mod volt_os;
pub mod volt_plugins;
pub mod volt_secure_storage;
pub mod volt_shell;
pub mod volt_tray;
pub mod volt_updater;
pub mod volt_watcher;
pub mod volt_window;

pub(crate) use event_helpers::{bind_native_event_off, bind_native_event_on};
pub use module_registry::{RegisteredModule, register_all_modules};
pub use runtime_state::{
    ModuleConfig, app_name, configure, fs_base_dir, plugin_manager, require_permission,
    require_permission_message, secure_storage_adapter, updater_telemetry_config,
};
pub use runtime_values::{
    bind_native_event_handler, format_js_error, js_error, json_to_js_value, native_function_module,
    normalize_single_event_name, promise_from_json_result, promise_from_result, reject_promise,
    resolve_promise, value_to_json,
};
