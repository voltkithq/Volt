use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use boa_engine::{JsNativeError, JsResult};
use volt_core::permissions::{CapabilityGuard, Permission};

use super::runtime_values::format_js_error;
use super::secure_storage;

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
