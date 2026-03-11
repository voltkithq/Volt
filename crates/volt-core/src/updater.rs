mod config;
mod http;
mod platform;
mod util;
mod verification;

pub use self::config::{UpdateConfig, UpdateInfo};
pub use self::platform::current_target;
pub use self::verification::UpdateError;

use self::http::{
    UPDATE_CHECK_RESPONSE_MAX_BYTES, build_http_client, fetch_with_validated_redirects,
    read_response_body_limited,
};
use self::util::{build_update_check_url, validate_url_security};

/// Check for available updates.
/// Returns `Ok(Some(info))` if an update is available, `Ok(None)` if up to date.
pub fn check_for_update(config: &UpdateConfig) -> Result<Option<UpdateInfo>, UpdateError> {
    let current = semver::Version::parse(&config.current_version)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid current version: {e}")))?;

    let target = current_target();
    let url = build_update_check_url(&config.endpoint, &config.current_version, target)?;

    validate_url_security(&url)?;

    let client = build_http_client().map_err(UpdateError::CheckFailed)?;
    let response = fetch_with_validated_redirects(&client, &url, "update check")
        .map_err(UpdateError::CheckFailed)?;
    let status = response.status();

    if status.as_u16() == 204 || status.as_u16() == 404 {
        return Ok(None);
    }

    if !status.is_success() {
        return Err(UpdateError::CheckFailed(format!(
            "server returned status {}",
            status
        )));
    }

    let body =
        read_response_body_limited(response, UPDATE_CHECK_RESPONSE_MAX_BYTES, "update check")
            .map_err(UpdateError::CheckFailed)?;
    let mut info: UpdateInfo = serde_json::from_slice(&body)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid response JSON: {e}")))?;
    info.target = target.to_string();

    let offered = semver::Version::parse(&info.version)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid offered version: {e}")))?;

    if offered <= current {
        return Err(UpdateError::DowngradePrevented {
            current: config.current_version.clone(),
            offered: info.version.clone(),
        });
    }

    validate_url_security(&info.url)?;
    verification::verify_update_metadata(config, &info)?;

    Ok(Some(info))
}

/// Download an update binary and verify its integrity.
/// Returns the verified binary data.
pub fn download_and_verify(
    config: &UpdateConfig,
    info: &UpdateInfo,
) -> Result<Vec<u8>, UpdateError> {
    verification::download_and_verify(config, info)
}

/// Apply an update by writing the new binary and replacing the current one.
pub fn apply_update(data: &[u8]) -> Result<(), UpdateError> {
    platform::apply_update(data)
}

#[cfg(test)]
mod tests;
