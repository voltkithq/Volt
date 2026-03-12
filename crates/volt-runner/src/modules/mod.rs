use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use boa_engine::module::{IntoJsModule, MapModuleLoader, Module};
use boa_engine::native_function::NativeFunction;
use boa_engine::object::JsObject;
use boa_engine::object::builtins::JsFunction;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsValue, js_string};
use volt_core::permissions::{CapabilityGuard, Permission};

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
pub mod volt_secure_storage;
pub mod volt_shell;
pub mod volt_tray;
pub mod volt_updater;
pub mod volt_window;

pub(crate) use event_helpers::{bind_native_event_off, bind_native_event_on};

#[derive(Debug, Clone)]
pub struct UpdaterTelemetryConfig {
    pub enabled: bool,
    pub sink: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleConfig {
    pub fs_base_dir: PathBuf,
    pub permissions: Vec<String>,
    pub app_name: String,
    pub secure_storage_backend: Option<String>,
    pub updater_telemetry_enabled: bool,
    pub updater_telemetry_sink: Option<String>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        let fs_base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            fs_base_dir,
            permissions: Vec::new(),
            app_name: "Volt App".to_string(),
            secure_storage_backend: None,
            updater_telemetry_enabled: false,
            updater_telemetry_sink: None,
        }
    }
}

struct ModuleState {
    fs_base_dir: PathBuf,
    app_name: String,
    permission_guard: CapabilityGuard,
    secure_storage_adapter: Arc<dyn secure_storage::SecureStorageAdapter>,
    updater_telemetry: UpdaterTelemetryConfig,
}

impl ModuleState {
    fn from_config(config: ModuleConfig) -> Self {
        let secure_storage_adapter = config.secure_storage_backend.as_deref().map_or_else(
            || secure_storage::create_secure_storage_adapter(&config.app_name),
            |backend| {
                secure_storage::create_secure_storage_adapter_with_override(
                    &config.app_name,
                    Some(backend),
                )
            },
        );

        Self {
            fs_base_dir: config.fs_base_dir,
            app_name: config.app_name,
            permission_guard: CapabilityGuard::from_names(&config.permissions),
            secure_storage_adapter,
            updater_telemetry: UpdaterTelemetryConfig {
                enabled: config.updater_telemetry_enabled,
                sink: config.updater_telemetry_sink,
            },
        }
    }
}

thread_local! {
    static MODULE_STATE: RefCell<ModuleState> =
        RefCell::new(ModuleState::from_config(ModuleConfig::default()));
}

pub fn configure(config: ModuleConfig) -> Result<(), String> {
    MODULE_STATE.with(|state| {
        *state.borrow_mut() = ModuleState::from_config(config);
    });
    Ok(())
}

pub fn fs_base_dir() -> Result<PathBuf, String> {
    MODULE_STATE.with(|state| Ok(state.borrow().fs_base_dir.clone()))
}

pub fn app_name() -> Result<String, String> {
    MODULE_STATE.with(|state| Ok(state.borrow().app_name.clone()))
}

pub fn secure_storage_adapter() -> Result<Arc<dyn secure_storage::SecureStorageAdapter>, String> {
    MODULE_STATE.with(|state| Ok(state.borrow().secure_storage_adapter.clone()))
}

pub fn updater_telemetry_config() -> Result<UpdaterTelemetryConfig, String> {
    MODULE_STATE.with(|state| Ok(state.borrow().updater_telemetry.clone()))
}

pub fn require_permission(permission: Permission) -> JsResult<()> {
    MODULE_STATE.with(|state| {
        let state = state.borrow();
        state.permission_guard.check(permission).map_err(|err| {
            JsNativeError::error()
                .with_message(format!(
                    "Permission denied: {err}. Add '{}' to permissions in volt.config.ts.",
                    permission.as_str()
                ))
                .into()
        })
    })
}

pub fn require_permission_message(permission: Permission) -> Result<(), String> {
    require_permission(permission).map_err(format_js_error)
}

pub fn js_error(
    _module: &'static str,
    _function: &'static str,
    message: impl Into<String>,
) -> JsError {
    JsNativeError::error().with_message(message.into()).into()
}

pub fn format_js_error(error: JsError) -> String {
    error.to_string()
}

pub fn normalize_single_event_name(
    feature_name: &str,
    event_name: String,
    accepted_event_name: &'static str,
    native_event_name: &'static str,
) -> Result<&'static str, String> {
    match event_name.trim() {
        name if name == accepted_event_name => Ok(native_event_name),
        "" => Err(format!("{feature_name} event name must not be empty")),
        other => Err(format!(
            "unsupported {feature_name} event '{other}', only '{accepted_event_name}' is supported"
        )),
    }
}

pub fn bind_native_event_handler(
    context: &mut Context,
    module_name: &'static str,
    api_function: &'static str,
    global_name: &'static str,
    event_type: &str,
    handler: JsFunction,
) -> JsResult<()> {
    let binder = context
        .global_object()
        .get(js_string!(global_name), context)
        .map_err(|error| {
            js_error(
                module_name,
                api_function,
                format!(
                    "native event bridge is unavailable: {}",
                    format_js_error(error)
                ),
            )
        })?;
    let binder = binder.as_callable().ok_or_else(|| {
        js_error(
            module_name,
            api_function,
            "native event bridge is unavailable: binder is not callable",
        )
    })?;

    binder
        .call(
            &JsValue::undefined(),
            &[JsValue::from(js_string!(event_type)), handler.into()],
            context,
        )
        .map(|_| ())
        .map_err(|error| {
            js_error(
                module_name,
                api_function,
                format!(
                    "failed to bind native event handler: {}",
                    format_js_error(error)
                ),
            )
        })
}

pub fn reject_promise(context: &mut Context, message: impl Into<String>) -> JsPromise {
    let message = message.into();
    JsPromise::reject(
        JsError::from_opaque(js_string!(message.as_str()).into()),
        context,
    )
}

pub trait IntoJsRuntimeValue {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String>;
}

macro_rules! impl_into_js_runtime_value_via_from {
    ($($type:ty),* $(,)?) => {
        $(
            impl IntoJsRuntimeValue for $type {
                fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
                    Ok(JsValue::from(self))
                }
            }
        )*
    };
}

impl_into_js_runtime_value_via_from!((), bool, i32, i64, u32, u64, usize, f64);

impl IntoJsRuntimeValue for JsValue {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(self)
    }
}

impl IntoJsRuntimeValue for JsObject {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(self.into())
    }
}

impl IntoJsRuntimeValue for String {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(js_string!(self.as_str()).into())
    }
}

impl IntoJsRuntimeValue for Option<String> {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        match self {
            Some(value) => Ok(js_string!(value.as_str()).into()),
            None => Ok(JsValue::null()),
        }
    }
}

impl IntoJsRuntimeValue for Vec<String> {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String> {
        json_to_js_value(
            &serde_json::Value::Array(self.into_iter().map(serde_json::Value::String).collect()),
            context,
        )
    }
}

impl IntoJsRuntimeValue for serde_json::Value {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String> {
        json_to_js_value(&self, context)
    }
}

pub fn resolve_promise<V: IntoJsRuntimeValue>(context: &mut Context, value: V) -> JsPromise {
    match value.into_js_runtime_value(context) {
        Ok(value) => JsPromise::resolve(value, context),
        Err(error) => reject_promise(context, error),
    }
}

pub fn promise_from_result<V: IntoJsRuntimeValue>(
    context: &mut Context,
    result: Result<V, String>,
) -> JsPromise {
    match result {
        Ok(value) => resolve_promise(context, value),
        Err(message) => reject_promise(context, message),
    }
}

pub fn json_to_js_value(
    value: &serde_json::Value,
    context: &mut Context,
) -> Result<JsValue, String> {
    JsValue::from_json(value, context).map_err(format_js_error)
}

pub fn value_to_json(value: JsValue, context: &mut Context) -> Result<serde_json::Value, String> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(serde_json::Value::Null))
        .map_err(format_js_error)
}

pub fn promise_from_json_result(
    context: &mut Context,
    result: Result<serde_json::Value, String>,
) -> JsPromise {
    match result {
        Ok(value) => match json_to_js_value(&value, context) {
            Ok(js_value) => resolve_promise(context, js_value),
            Err(error) => reject_promise(context, error),
        },
        Err(error) => reject_promise(context, error),
    }
}

pub fn native_function_module(
    context: &mut Context,
    exports: Vec<(&'static str, NativeFunction)>,
) -> Module {
    exports
        .into_iter()
        .map(|(name, function)| (js_string!(name), function))
        .collect::<Vec<_>>()
        .into_js_module(context)
}

pub struct RegisteredModule {
    pub global_name: &'static str,
    pub specifier: &'static str,
    pub module: Module,
}

pub fn register_all_modules(
    context: &mut Context,
    module_loader: &MapModuleLoader,
) -> JsResult<Vec<RegisteredModule>> {
    let mut registered_modules = Vec::with_capacity(18);

    let fs = volt_fs::build_module(context);
    module_loader.insert("volt:fs", fs.clone());
    registered_modules.push(RegisteredModule {
        global_name: "fs",
        specifier: "volt:fs",
        module: fs,
    });

    let db = volt_db::build_module(context);
    module_loader.insert("volt:db", db.clone());
    registered_modules.push(RegisteredModule {
        global_name: "db",
        specifier: "volt:db",
        module: db,
    });

    let bench = volt_bench::build_module(context);
    module_loader.insert("volt:bench", bench.clone());
    registered_modules.push(RegisteredModule {
        global_name: "bench",
        specifier: "volt:bench",
        module: bench,
    });

    let dialog = volt_dialog::build_module(context);
    module_loader.insert("volt:dialog", dialog.clone());
    registered_modules.push(RegisteredModule {
        global_name: "dialog",
        specifier: "volt:dialog",
        module: dialog,
    });

    let events = volt_events::build_module(context);
    module_loader.insert("volt:events", events.clone());
    registered_modules.push(RegisteredModule {
        global_name: "events",
        specifier: "volt:events",
        module: events,
    });

    let clipboard = volt_clipboard::build_module(context);
    module_loader.insert("volt:clipboard", clipboard.clone());
    registered_modules.push(RegisteredModule {
        global_name: "clipboard",
        specifier: "volt:clipboard",
        module: clipboard,
    });

    let shell = volt_shell::build_module(context);
    module_loader.insert("volt:shell", shell.clone());
    registered_modules.push(RegisteredModule {
        global_name: "shell",
        specifier: "volt:shell",
        module: shell,
    });

    let notification = volt_notification::build_module(context);
    module_loader.insert("volt:notification", notification.clone());
    registered_modules.push(RegisteredModule {
        global_name: "notification",
        specifier: "volt:notification",
        module: notification,
    });

    let menu = volt_menu::build_module(context);
    module_loader.insert("volt:menu", menu.clone());
    registered_modules.push(RegisteredModule {
        global_name: "menu",
        specifier: "volt:menu",
        module: menu,
    });

    let http = volt_http::build_module(context);
    module_loader.insert("volt:http", http.clone());
    registered_modules.push(RegisteredModule {
        global_name: "http",
        specifier: "volt:http",
        module: http,
    });

    let global_shortcut = volt_global_shortcut::build_module(context);
    module_loader.insert("volt:globalShortcut", global_shortcut.clone());
    registered_modules.push(RegisteredModule {
        global_name: "globalShortcut",
        specifier: "volt:globalShortcut",
        module: global_shortcut,
    });

    let ipc = volt_ipc::build_module(context)?;
    module_loader.insert("volt:ipc", ipc.clone());
    registered_modules.push(RegisteredModule {
        global_name: "ipc",
        specifier: "volt:ipc",
        module: ipc,
    });

    let secure_storage = volt_secure_storage::build_module(context);
    module_loader.insert("volt:secureStorage", secure_storage.clone());
    registered_modules.push(RegisteredModule {
        global_name: "secureStorage",
        specifier: "volt:secureStorage",
        module: secure_storage,
    });

    let tray = volt_tray::build_module(context);
    module_loader.insert("volt:tray", tray.clone());
    registered_modules.push(RegisteredModule {
        global_name: "tray",
        specifier: "volt:tray",
        module: tray,
    });

    let crypto = volt_crypto::build_module(context);
    module_loader.insert("volt:crypto", crypto.clone());
    registered_modules.push(RegisteredModule {
        global_name: "crypto",
        specifier: "volt:crypto",
        module: crypto,
    });

    let updater = volt_updater::build_module(context);
    module_loader.insert("volt:updater", updater.clone());
    registered_modules.push(RegisteredModule {
        global_name: "updater",
        specifier: "volt:updater",
        module: updater,
    });

    let window = volt_window::build_module(context);
    module_loader.insert("volt:window", window.clone());
    registered_modules.push(RegisteredModule {
        global_name: "window",
        specifier: "volt:window",
        module: window,
    });

    let os = volt_os::build_module(context);
    module_loader.insert("volt:os", os.clone());
    registered_modules.push(RegisteredModule {
        global_name: "os",
        specifier: "volt:os",
        module: os,
    });

    Ok(registered_modules)
}
