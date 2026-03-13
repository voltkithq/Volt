use napi_derive::napi;
use serde_json::Value;
use volt_core::dialog;
use volt_core::grant_store;
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

/// Result from a grant-aware open dialog.
#[napi(object)]
pub struct GrantDialogResult {
    /// Selected file/directory paths.
    pub paths: Vec<String>,
    /// Grant IDs corresponding to each selected path (only for directories).
    pub grant_ids: Vec<String>,
}

/// Show an open folder dialog that creates filesystem scope grants for selected directories.
/// Requires both `dialog` and `fs` permissions.
/// Returns paths and corresponding grant IDs, or empty arrays if cancelled.
#[napi]
pub fn dialog_show_open_with_grant(options: Value) -> napi::Result<GrantDialogResult> {
    require_permission(Permission::Dialog)?;
    require_permission(Permission::FileSystem)?;

    let mut opts: dialog::OpenDialogOptions = serde_json::from_value(options)
        .map_err(|e| napi::Error::from_reason(format!("Invalid open dialog options: {e}")))?;

    // Force directory mode for grant dialogs
    opts.directory = true;

    let selected = dialog::show_open_dialog(&opts);
    let mut paths = Vec::new();
    let mut grant_ids = Vec::new();

    for path in selected {
        let path_str = path.to_string_lossy().into_owned();
        match grant_store::create_grant(path) {
            Ok(grant_id) => {
                paths.push(path_str);
                grant_ids.push(grant_id);
            }
            Err(e) => {
                return Err(napi::Error::from_reason(format!(
                    "Failed to create grant for selected path: {e}"
                )));
            }
        }
    }

    Ok(GrantDialogResult { paths, grant_ids })
}

/// Show a message dialog. Returns true if user confirmed, false otherwise.
#[napi]
pub fn dialog_show_message(options: Value) -> napi::Result<bool> {
    require_permission(Permission::Dialog)?;
    let opts: dialog::MessageDialogOptions = serde_json::from_value(options)
        .map_err(|e| napi::Error::from_reason(format!("Invalid message dialog options: {e}")))?;

    Ok(dialog::show_message_dialog(&opts))
}
