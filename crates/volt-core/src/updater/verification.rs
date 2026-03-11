use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::config::{UpdateConfig, UpdateInfo};
use super::http::{
    UPDATE_DOWNLOAD_MAX_BYTES, build_http_client, fetch_with_validated_redirects,
    read_response_body_limited,
};
use super::util::{hex_encode, validate_url_security};

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("update check failed: {0}")]
    CheckFailed(String),

    #[error("download failed: {0}")]
    DownloadFailed(String),

    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),

    #[error("SHA-256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("downgrade prevented: current {current}, offered {offered}")]
    DowngradePrevented { current: String, offered: String },

    #[error("insecure URL: {0}")]
    InsecureUrl(String),

    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("apply failed: {0}")]
    ApplyFailed(String),
}

/// Decode a base64-encoded Ed25519 public key.
pub(super) fn decode_public_key(key_b64: &str) -> Result<VerifyingKey, UpdateError> {
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(key_b64)
        .map_err(|e| UpdateError::InvalidPublicKey(format!("base64 decode: {e}")))?;

    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| UpdateError::InvalidPublicKey("key must be exactly 32 bytes".to_string()))?;

    VerifyingKey::from_bytes(&key_array)
        .map_err(|e| UpdateError::InvalidPublicKey(format!("invalid key: {e}")))
}

pub(super) fn canonical_update_metadata_payload(info: &UpdateInfo) -> Vec<u8> {
    let normalized_sha256 = info.sha256.trim().to_ascii_lowercase();
    let mut payload = Vec::with_capacity(
        "volt-update-v1".len()
            + info.version.len()
            + info.url.len()
            + normalized_sha256.len()
            + info.target.len()
            + 5,
    );
    payload.extend_from_slice(b"volt-update-v1\0");
    payload.extend_from_slice(info.version.as_bytes());
    payload.push(0);
    payload.extend_from_slice(info.url.as_bytes());
    payload.push(0);
    payload.extend_from_slice(normalized_sha256.as_bytes());
    payload.push(0);
    payload.extend_from_slice(info.target.as_bytes());
    payload
}

pub(super) fn verify_update_metadata(
    config: &UpdateConfig,
    info: &UpdateInfo,
) -> Result<(), UpdateError> {
    let public_key = decode_public_key(&config.public_key)?;
    verify_update_version(config, info)?;
    verify_metadata_signature(&public_key, info)
}

pub(super) fn download_and_verify(
    config: &UpdateConfig,
    info: &UpdateInfo,
) -> Result<Vec<u8>, UpdateError> {
    let public_key = decode_public_key(&config.public_key)?;
    verify_update_version(config, info)?;
    verify_metadata_signature(&public_key, info)?;

    validate_url_security(&info.url)?;

    let client = build_http_client().map_err(UpdateError::DownloadFailed)?;
    let response = fetch_with_validated_redirects(&client, &info.url, "update download")
        .map_err(UpdateError::DownloadFailed)?;

    if !response.status().is_success() {
        return Err(UpdateError::DownloadFailed(format!(
            "server returned status {}",
            response.status()
        )));
    }

    let data = read_response_body_limited(response, UPDATE_DOWNLOAD_MAX_BYTES, "update download")
        .map_err(UpdateError::DownloadFailed)?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    let actual_hash = hex_encode(&hash);

    let expected_hash = info.sha256.trim().to_ascii_lowercase();
    if actual_hash != expected_hash {
        return Err(UpdateError::ChecksumMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }

    Ok(data)
}

fn verify_update_version(config: &UpdateConfig, info: &UpdateInfo) -> Result<(), UpdateError> {
    let current = semver::Version::parse(&config.current_version)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid current version: {e}")))?;
    let offered = semver::Version::parse(&info.version)
        .map_err(|e| UpdateError::CheckFailed(format!("invalid offered version: {e}")))?;

    if offered <= current {
        return Err(UpdateError::DowngradePrevented {
            current: config.current_version.clone(),
            offered: info.version.clone(),
        });
    }

    Ok(())
}

fn verify_metadata_signature(
    public_key: &VerifyingKey,
    info: &UpdateInfo,
) -> Result<(), UpdateError> {
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&info.signature)
        .map_err(|e| UpdateError::SignatureInvalid(format!("base64 decode: {e}")))?;

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| UpdateError::SignatureInvalid(format!("invalid signature format: {e}")))?;

    let payload = canonical_update_metadata_payload(info);
    public_key
        .verify(&payload, &signature)
        .map_err(|e| UpdateError::SignatureInvalid(format!("verification failed: {e}")))
}
