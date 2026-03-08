use sha2::{Digest, Sha256};
use url::Url;

use super::verification::UpdateError;

/// Encode bytes as lowercase hex string.
pub(super) fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

/// Validate that a URL uses HTTPS (or is localhost for testing).
pub(super) fn validate_url_security(url_str: &str) -> Result<(), UpdateError> {
    let parsed =
        Url::parse(url_str).map_err(|e| UpdateError::InsecureUrl(format!("invalid URL: {e}")))?;

    let scheme = parsed.scheme();
    if scheme == "https" {
        return Ok(());
    }

    if scheme == "http" {
        let is_localhost = match parsed.host() {
            Some(url::Host::Domain(d)) => d == "localhost",
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        };
        if is_localhost {
            return Ok(());
        }
    }

    Err(UpdateError::InsecureUrl(format!(
        "URL must use HTTPS: {url_str}"
    )))
}

pub(super) fn build_update_check_url(
    endpoint: &str,
    current_version: &str,
    target: &str,
) -> Result<String, UpdateError> {
    let mut parsed = url::Url::parse(endpoint)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid endpoint URL: {e}")))?;
    parsed
        .query_pairs_mut()
        .append_pair("current_version", current_version)
        .append_pair("target", target);
    Ok(parsed.to_string())
}
