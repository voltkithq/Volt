use napi_derive::napi;
use volt_core::command::{AppCommand, send_command, send_query};

/// Close a window by its JS ID.
#[napi]
pub fn window_close(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::CloseWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Show a window by its JS ID.
#[napi]
pub fn window_show(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::ShowWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Focus a window by its JS ID.
#[napi]
pub fn window_focus(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::FocusWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Maximize a window by its JS ID.
#[napi]
pub fn window_maximize(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::MaximizeWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Minimize a window by its JS ID.
#[napi]
pub fn window_minimize(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::MinimizeWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Restore a window by its JS ID.
#[napi]
pub fn window_restore(js_id: String) -> napi::Result<()> {
    send_command(AppCommand::RestoreWindow { js_id })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Execute a JavaScript string in a window's webview.
#[napi]
pub fn window_eval_script(js_id: String, script: String) -> napi::Result<()> {
    send_command(AppCommand::EvaluateScript { js_id, script })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Get the number of tracked windows.
#[napi]
pub fn window_count() -> napi::Result<u32> {
    send_query(|reply| AppCommand::GetWindowCount { reply })
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}
