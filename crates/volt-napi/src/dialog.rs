use napi_derive::napi;
use serde_json::Value;
use volt_core::dialog;
use volt_core::permissions::Permission;

use crate::permissions::require_permission;

/// Show an open file/folder dialog. Returns selected paths, or empty array if cancelled.
#[napi]
pub fn dialog_show_open(options: Value) -> napi::Result<Vec<String>> {
    require_permission(Permission::Dialog)?;
    let opts: dialog::OpenDialogOptions = serde_json::from_value(options)
        .map_err(|e| napi::Error::from_reason(format!("Invalid open dialog options: {e}")))?;

    let paths = dialog::show_open_dialog(&opts);
    Ok(paths
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect())
}

/// Show a save file dialog. Returns the selected path, or null if cancelled.
#[napi]
pub fn dialog_show_save(options: Value) -> napi::Result<Option<String>> {
    require_permission(Permission::Dialog)?;
    let opts: dialog::SaveDialogOptions = serde_json::from_value(options)
        .map_err(|e| napi::Error::from_reason(format!("Invalid save dialog options: {e}")))?;

    Ok(dialog::show_save_dialog(&opts).map(|p| p.to_string_lossy().into_owned()))
}

/// Show a message dialog. Returns true if user confirmed, false otherwise.
#[napi]
pub fn dialog_show_message(options: Value) -> napi::Result<bool> {
    require_permission(Permission::Dialog)?;
    let opts: dialog::MessageDialogOptions = serde_json::from_value(options)
        .map_err(|e| napi::Error::from_reason(format!("Invalid message dialog options: {e}")))?;

    Ok(dialog::show_message_dialog(&opts))
}
