use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Mutex, Once, OnceLock};
use volt_core::permissions::Permission;
use volt_core::tray::{TrayConfig, TrayHandle};

use crate::permissions::require_permission;

static TRAY_CLICK_CALLBACKS: OnceLock<Mutex<HashMap<String, ThreadsafeFunction<String>>>> =
    OnceLock::new();
static TRAY_EVENT_HANDLER_INIT: Once = Once::new();

fn callback_registry() -> &'static Mutex<HashMap<String, ThreadsafeFunction<String>>> {
    TRAY_CLICK_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn ensure_tray_event_handler() {
    TRAY_EVENT_HANDLER_INIT.call_once(|| {
        tray_icon::TrayIconEvent::set_event_handler(Some(|event| {
            if !matches!(event, tray_icon::TrayIconEvent::Click { .. }) {
                return;
            }

            let tray_id = event.id().as_ref().to_string();
            if let Ok(callbacks) = callback_registry().lock()
                && let Some(callback) = callbacks.get(&tray_id)
            {
                let payload = json!({
                    "type": "click",
                    "trayId": tray_id,
                })
                .to_string();
                let _ = callback.call(Ok(payload), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }));
    });
}

fn unregister_click_callback(tray_id: &str) {
    if let Ok(mut callbacks) = callback_registry().lock() {
        callbacks.remove(tray_id);
    }
}

fn load_icon_rgba(path: &str) -> napi::Result<(Vec<u8>, u32, u32)> {
    require_permission(Permission::FileSystem)?;
    let image = image::open(path)
        .map_err(|e| napi::Error::from_reason(format!("Failed to load icon '{path}': {e}")))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

/// JavaScript-facing system tray class.
#[napi]
pub struct VoltTray {
    handle: Option<TrayHandle>,
    tray_id: String,
}

#[napi]
impl VoltTray {
    /// Create a new system tray icon from a JSON configuration.
    #[napi(constructor)]
    pub fn new(config: Value) -> napi::Result<Self> {
        require_permission(Permission::Tray)?;

        let tooltip = config
            .get("tooltip")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let icon = config
            .get("icon")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let (icon, icon_width, icon_height) = if let Some(path) = icon {
            let (rgba, width, height) = load_icon_rgba(&path)?;
            (Some(rgba), width, height)
        } else {
            (None, 32, 32)
        };

        let tray_config = TrayConfig {
            tooltip,
            icon,
            icon_width,
            icon_height,
        };

        let handle = TrayHandle::new(&tray_config)
            .map_err(|e| napi::Error::from_reason(format!("Failed to create tray: {e}")))?;
        let tray_id = handle.id().to_string();

        Ok(Self {
            handle: Some(handle),
            tray_id,
        })
    }

    /// Set the tray tooltip text.
    #[napi]
    pub fn set_tooltip(&self, tooltip: String) -> napi::Result<()> {
        require_permission(Permission::Tray)?;
        if let Some(ref handle) = self.handle {
            handle
                .set_tooltip(&tooltip)
                .map_err(|e| napi::Error::from_reason(format!("Failed to set tooltip: {e}")))?;
        }
        Ok(())
    }

    /// Set the tray icon from an image file path.
    #[napi]
    pub fn set_icon(&self, icon_path: String) -> napi::Result<()> {
        require_permission(Permission::Tray)?;
        if let Some(ref handle) = self.handle {
            let (rgba, width, height) = load_icon_rgba(&icon_path)?;
            handle
                .set_icon(rgba, width, height)
                .map_err(|e| napi::Error::from_reason(format!("Failed to set tray icon: {e}")))?;
        }
        Ok(())
    }

    /// Set the tray visibility.
    #[napi]
    pub fn set_visible(&self, visible: bool) -> napi::Result<()> {
        require_permission(Permission::Tray)?;
        if let Some(ref handle) = self.handle {
            handle
                .set_visible(visible)
                .map_err(|e| napi::Error::from_reason(format!("Failed to set visibility: {e}")))?;
        }
        Ok(())
    }

    /// Register a click callback. Receives a JSON string with event details.
    #[napi]
    pub fn on_click(&mut self, callback: ThreadsafeFunction<String>) -> napi::Result<()> {
        require_permission(Permission::Tray)?;
        ensure_tray_event_handler();
        let mut callbacks = callback_registry()
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Tray callback lock poisoned: {e}")))?;
        callbacks.insert(self.tray_id.clone(), callback);
        Ok(())
    }

    /// Destroy the tray icon and clean up resources.
    #[napi]
    pub fn destroy(&mut self) -> napi::Result<()> {
        require_permission(Permission::Tray)?;
        unregister_click_callback(&self.tray_id);
        self.handle = None;
        Ok(())
    }
}

impl Drop for VoltTray {
    fn drop(&mut self) {
        unregister_click_callback(&self.tray_id);
        self.handle = None;
    }
}
