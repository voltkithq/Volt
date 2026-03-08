use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use volt_core::command::{AppCommand, send_query};
use volt_core::permissions::Permission;

use crate::permissions::require_permission;

struct ShortcutCallbackEntry {
    accelerator: String,
    callback: ThreadsafeFunction<String>,
}

static SHORTCUT_CALLBACKS: OnceLock<Mutex<HashMap<u32, Vec<ShortcutCallbackEntry>>>> =
    OnceLock::new();

fn callback_registry() -> &'static Mutex<HashMap<u32, Vec<ShortcutCallbackEntry>>> {
    SHORTCUT_CALLBACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn add_callback(
    shortcut_id: u32,
    accelerator: String,
    callback: ThreadsafeFunction<String>,
) -> napi::Result<()> {
    let mut guard = callback_registry()
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Shortcut callback lock poisoned: {e}")))?;
    guard
        .entry(shortcut_id)
        .or_default()
        .push(ShortcutCallbackEntry {
            accelerator,
            callback,
        });
    Ok(())
}

fn remove_callback(shortcut_id: u32, accelerator: &str) -> napi::Result<()> {
    let mut guard = callback_registry()
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Shortcut callback lock poisoned: {e}")))?;

    if let Some(entries) = guard.get_mut(&shortcut_id) {
        entries.retain(|entry| entry.accelerator != accelerator);
        if entries.is_empty() {
            guard.remove(&shortcut_id);
        }
    }
    Ok(())
}

pub(crate) fn clear_shortcut_callbacks() {
    if let Ok(mut guard) = callback_registry().lock() {
        guard.clear();
    }
}

/// Called from VoltApp's event bridge when the native loop emits a shortcut event.
pub(crate) fn dispatch_shortcut_trigger(shortcut_id: u32) {
    if let Ok(guard) = callback_registry().lock()
        && let Some(entries) = guard.get(&shortcut_id)
    {
        for entry in entries {
            let status = entry.callback.call(
                Ok(entry.accelerator.clone()),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
            if status != napi::Status::Ok {
                tracing::warn!(
                    accelerator = %entry.accelerator,
                    status = ?status,
                    "failed to dispatch shortcut callback"
                );
            }
        }
    }
}

/// JavaScript-facing global shortcut manager.
#[napi]
pub struct VoltGlobalShortcut {
    registrations: Vec<(String, u32)>,
}

#[napi]
impl VoltGlobalShortcut {
    /// Create a new global shortcut manager.
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        require_permission(Permission::GlobalShortcut)?;
        Ok(Self {
            registrations: Vec::new(),
        })
    }

    /// Register a global shortcut with an accelerator string (e.g., "CmdOrCtrl+Shift+P").
    /// The callback receives the accelerator string when triggered.
    #[napi]
    pub fn register(
        &mut self,
        accelerator: String,
        callback: ThreadsafeFunction<String>,
    ) -> napi::Result<()> {
        require_permission(Permission::GlobalShortcut)?;
        if self
            .registrations
            .iter()
            .any(|(registered, _)| registered == &accelerator)
        {
            return Err(napi::Error::from_reason(format!(
                "Shortcut '{accelerator}' is already registered"
            )));
        }
        let shortcut_id = send_query(|reply| AppCommand::RegisterShortcut {
            accelerator: accelerator.clone(),
            reply,
        })
        .map_err(|err| napi::Error::from_reason(err.to_string()))?
        .map_err(napi::Error::from_reason)?;

        add_callback(shortcut_id, accelerator.clone(), callback)?;
        self.registrations.push((accelerator, shortcut_id));
        Ok(())
    }

    /// Unregister a previously registered global shortcut.
    #[napi]
    pub fn unregister(&mut self, accelerator: String) -> napi::Result<()> {
        require_permission(Permission::GlobalShortcut)?;
        send_query(|reply| AppCommand::UnregisterShortcut {
            accelerator: accelerator.clone(),
            reply,
        })
        .map_err(|err| napi::Error::from_reason(err.to_string()))?
        .map_err(napi::Error::from_reason)?;

        if let Some(index) = self
            .registrations
            .iter()
            .position(|(registered, _)| registered == &accelerator)
        {
            let (_, shortcut_id) = self.registrations.remove(index);
            remove_callback(shortcut_id, &accelerator)?;
        }

        Ok(())
    }

    /// Unregister all global shortcuts.
    #[napi]
    pub fn unregister_all(&mut self) -> napi::Result<()> {
        require_permission(Permission::GlobalShortcut)?;
        send_query(|reply| AppCommand::UnregisterAllShortcuts { reply })
            .map_err(|err| napi::Error::from_reason(err.to_string()))?
            .map_err(napi::Error::from_reason)?;

        for (accelerator, shortcut_id) in self.registrations.drain(..) {
            remove_callback(shortcut_id, &accelerator)?;
        }

        Ok(())
    }

    /// Check if a shortcut is registered.
    #[napi]
    pub fn is_registered(&self, accelerator: String) -> bool {
        self.registrations
            .iter()
            .any(|(registered, _)| registered == &accelerator)
    }

    /// Get all registered accelerator strings.
    #[napi]
    pub fn get_registered(&self) -> Vec<String> {
        self.registrations
            .iter()
            .map(|(accelerator, _)| accelerator.clone())
            .collect()
    }
}
