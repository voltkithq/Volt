use napi::bindgen_prelude::{Buffer, spawn_blocking};
use napi_derive::napi;
use serde_json::Value;
use volt_core::permissions::Permission;
use volt_core::updater::{self, UpdateConfig, UpdateInfo};

use crate::permissions::require_permission;

fn require_updater_permissions() -> napi::Result<()> {
    require_permission(Permission::FileSystem)?;
    require_permission(Permission::Http)
}

/// Check for available updates.
/// Returns update info if available, or null if up to date.
#[napi]
pub async fn updater_check(config: Value) -> napi::Result<Option<Value>> {
    require_updater_permissions()?;
    let cfg: UpdateConfig = serde_json::from_value(config)
        .map_err(|e| napi::Error::from_reason(format!("Invalid update config: {e}")))?;

    let result = spawn_blocking(move || updater::check_for_update(&cfg))
        .await
        .map_err(|e| napi::Error::from_reason(format!("Update check task failed: {e}")))?
        .map_err(|e| napi::Error::from_reason(format!("Update check failed: {e}")))?;

    result
        .map(|info| {
            serde_json::to_value(&info).map_err(|e| {
                napi::Error::from_reason(format!("Failed to serialize update info: {e}"))
            })
        })
        .transpose()
}

/// Download an update and verify its SHA-256 checksum and Ed25519 signature.
/// Returns the verified binary data as a Buffer.
#[napi]
pub async fn updater_download_and_verify(config: Value, info: Value) -> napi::Result<Buffer> {
    require_updater_permissions()?;
    let cfg: UpdateConfig = serde_json::from_value(config)
        .map_err(|e| napi::Error::from_reason(format!("Invalid update config: {e}")))?;

    let update_info: UpdateInfo = serde_json::from_value(info)
        .map_err(|e| napi::Error::from_reason(format!("Invalid update info: {e}")))?;

    let data = spawn_blocking(move || updater::download_and_verify(&cfg, &update_info))
        .await
        .map_err(|e| napi::Error::from_reason(format!("Update download task failed: {e}")))?
        .map_err(|e| napi::Error::from_reason(format!("Download and verify failed: {e}")))?;

    Ok(data.into())
}

/// Apply a downloaded update by replacing the current binary.
#[napi]
pub fn updater_apply(data: Buffer) -> napi::Result<()> {
    require_updater_permissions()?;
    updater::apply_update(&data)
        .map_err(|e| napi::Error::from_reason(format!("Apply update failed: {e}")))
}
