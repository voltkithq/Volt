use serde::{Deserialize, Serialize};

/// Configuration for the auto-updater.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// URL endpoint to check for updates.
    pub endpoint: String,
    /// Ed25519 public key (base64-encoded, 32 bytes).
    pub public_key: String,
    /// Current application version (semver).
    pub current_version: String,
}

/// Information about an available update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// New version string (semver).
    pub version: String,
    /// Download URL for the update binary.
    pub url: String,
    /// Ed25519 signature of the canonical update metadata payload
    /// (`version`, `url`, `sha256`) encoded as base64.
    pub signature: String,
    /// SHA-256 hash of the binary (hex-encoded).
    pub sha256: String,
}
