use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use volt_core::command::{self, AppCommand};

#[derive(Debug)]
struct UpdateInstallState {
    next_operation_id: u64,
    active_operation_id: Option<u64>,
    cancelled_operation_id: Option<u64>,
}

impl Default for UpdateInstallState {
    fn default() -> Self {
        Self {
            next_operation_id: 1,
            active_operation_id: None,
            cancelled_operation_id: None,
        }
    }
}

static UPDATE_INSTALL_STATE: OnceLock<Mutex<UpdateInstallState>> = OnceLock::new();

fn install_state_slot() -> &'static Mutex<UpdateInstallState> {
    UPDATE_INSTALL_STATE.get_or_init(|| Mutex::new(UpdateInstallState::default()))
}

pub(crate) fn emit_update_ready_event(version: &str) -> Result<(), String> {
    command::send_command(AppCommand::EmitEvent {
        js_window_id: None,
        event_name: "update:ready".to_string(),
        data: json!({ "version": version }),
    })
    .map_err(|error| format!("failed to emit update:ready event: {error}"))
}

pub(crate) fn emit_update_progress_event(
    version: &str,
    stage: &str,
    percent: u8,
) -> Result<(), String> {
    command::send_command(AppCommand::EmitEvent {
        js_window_id: None,
        event_name: "update:progress".to_string(),
        data: json!({
            "version": version,
            "stage": stage,
            "percent": percent,
        }),
    })
    .map_err(|error| format!("failed to emit update:progress event: {error}"))
}

/// Emit updater lifecycle telemetry according to the configured data policy.
/// Policy: only stage/status/version/timestamp/detail are emitted. No user data.
pub(crate) fn emit_update_lifecycle_telemetry(
    version: &str,
    stage: &str,
    status: &str,
    detail: Option<&str>,
) -> Result<(), String> {
    let telemetry =
        crate::modules::updater_telemetry_config().map_err(|error| error.to_string())?;
    if !telemetry.enabled {
        return Ok(());
    }

    let payload = json!({
        "schemaVersion": 1,
        "event": "update:lifecycle",
        "version": version,
        "stage": stage,
        "status": status,
        "timestampUnixMs": now_unix_ms()?,
        "detail": detail.unwrap_or(""),
    });

    command::send_command(AppCommand::EmitEvent {
        js_window_id: None,
        event_name: "update:telemetry".to_string(),
        data: payload.clone(),
    })
    .map_err(|error| format!("failed to emit update:telemetry event: {error}"))?;

    if telemetry
        .sink
        .as_deref()
        .unwrap_or("none")
        .trim()
        .to_ascii_lowercase()
        .as_str()
        == "stdout"
    {
        println!("[volt][update-telemetry] {payload}");
    }

    Ok(())
}

pub(crate) fn begin_update_install_operation() -> Result<u64, String> {
    let mut state = install_state_slot()
        .lock()
        .map_err(|error| format!("update install state lock poisoned: {error}"))?;
    if state.active_operation_id.is_some() {
        return Err("update installation already in progress".to_string());
    }

    let operation_id = state.next_operation_id;
    state.next_operation_id = state.next_operation_id.saturating_add(1).max(1);
    state.active_operation_id = Some(operation_id);
    state.cancelled_operation_id = None;
    Ok(operation_id)
}

pub(crate) fn finish_update_install_operation(operation_id: u64) {
    if let Ok(mut state) = install_state_slot().lock() {
        if state.active_operation_id == Some(operation_id) {
            state.active_operation_id = None;
        }
        if state.cancelled_operation_id == Some(operation_id) {
            state.cancelled_operation_id = None;
        }
    }
}

pub(crate) fn is_update_install_cancelled(operation_id: u64) -> bool {
    match install_state_slot().lock() {
        Ok(state) => state.cancelled_operation_id == Some(operation_id),
        Err(_) => false,
    }
}

pub(crate) fn mark_active_update_install_cancelled() {
    if let Ok(mut state) = install_state_slot().lock()
        && let Some(operation_id) = state.active_operation_id
    {
        state.cancelled_operation_id = Some(operation_id);
    }
}

#[cfg(test)]
pub(crate) fn reset_update_install_state_for_tests() {
    if let Ok(mut state) = install_state_slot().lock() {
        state.active_operation_id = None;
        state.cancelled_operation_id = None;
    }
}

fn now_unix_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?;
    Ok(elapsed.as_millis() as u64)
}
