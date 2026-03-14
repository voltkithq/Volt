use napi_derive::napi;
use std::path::Path;
use volt_core::permissions::Permission;
use volt_core::shell;

use crate::permissions::require_permission;

/// Open a URL in the default system application.
/// Only http, https, and mailto schemes are allowed.
#[napi]
pub fn shell_open_external(url: String) -> napi::Result<()> {
    require_permission(Permission::Shell)?;
    shell::open_external(&url)
        .map_err(|e| napi::Error::from_reason(format!("shell open failed: {e}")))
}

/// Reveal a file or directory in the platform file manager.
#[napi]
pub fn shell_show_item_in_folder(path: String) -> napi::Result<()> {
    require_permission(Permission::Shell)?;
    shell::show_item_in_folder(Path::new(&path))
        .map_err(|e| napi::Error::from_reason(format!("show item in folder failed: {e}")))
}
