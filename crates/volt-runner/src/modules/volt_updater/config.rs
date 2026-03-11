use serde_json::Value;
use volt_core::permissions::Permission;
use volt_core::updater::{self, UpdateConfig, UpdateInfo};

use super::super::require_permission;

pub(crate) const EMBEDDED_UPDATE_PUBLIC_KEY: &str =
    include_str!(concat!(env!("OUT_DIR"), "/embedded-update-public-key.txt"));

#[derive(Debug, Clone)]
pub(crate) struct UpdateCheckOptions {
    pub(crate) url: String,
    pub(crate) current_version: String,
}

pub(crate) fn normalize_non_empty(value: String, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn embedded_update_public_key() -> Result<String, String> {
    let key = EMBEDDED_UPDATE_PUBLIC_KEY.trim();
    if key.is_empty() {
        return Err(
            "updater public key is not embedded. Set updater.publicKey in volt.config.ts before building."
                .to_string(),
        );
    }
    Ok(key.to_string())
}

pub(crate) fn ensure_updater_permissions() -> Result<(), String> {
    require_permission(Permission::FileSystem).map_err(|error| error.to_string())?;
    require_permission(Permission::Http).map_err(|error| error.to_string())
}

pub(crate) fn parse_check_options_json(value: Value) -> Result<UpdateCheckOptions, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "invalid update options: expected an object".to_string())?;
    let url = object
        .get("url")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "invalid update options: missing 'url'".to_string())?;
    let current_version = object
        .get("currentVersion")
        .and_then(Value::as_str)
        .or_else(|| object.get("current_version").and_then(Value::as_str))
        .map(ToString::to_string)
        .ok_or_else(|| "invalid update options: missing 'currentVersion'".to_string())?;

    Ok(UpdateCheckOptions {
        url: normalize_non_empty(url, "options.url")?,
        current_version: normalize_non_empty(current_version, "options.currentVersion")?,
    })
}

pub(crate) fn parse_update_info_json(value: Value) -> Result<UpdateInfo, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "invalid update info: expected an object".to_string())?;

    let read_required = |key: &'static str| -> Result<String, String> {
        let value = object
            .get(key)
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| format!("invalid update info: missing '{key}'"))?;
        normalize_non_empty(value, &format!("updateInfo.{key}"))
    };

    Ok(UpdateInfo {
        version: read_required("version")?,
        url: read_required("url")?,
        signature: read_required("signature")?,
        sha256: read_required("sha256")?,
        target: object
            .get("target")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

pub(crate) fn ensure_update_version_is_newer(version: &str) -> Result<(), String> {
    let current = semver::Version::parse(&current_app_version())
        .map_err(|error| format!("invalid current application version: {error}"))?;
    let offered = semver::Version::parse(version)
        .map_err(|error| format!("invalid offered update version: {error}"))?;

    if offered <= current {
        return Err(format!(
            "update version '{}' must be greater than current version '{}'",
            version, current
        ));
    }

    Ok(())
}

pub(crate) fn build_update_config(
    options: &UpdateCheckOptions,
    public_key: String,
) -> UpdateConfig {
    UpdateConfig {
        endpoint: options.url.clone(),
        public_key,
        current_version: options.current_version.clone(),
    }
}

pub(crate) fn check_for_update_with_public_key(
    options: UpdateCheckOptions,
    public_key: String,
) -> Result<Option<UpdateInfo>, String> {
    let config = build_update_config(&options, public_key);
    updater::check_for_update(&config).map_err(|error| format!("update check failed: {error}"))
}

pub(crate) fn current_app_version() -> String {
    option_env!("VOLT_APP_VERSION")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .to_string()
}
