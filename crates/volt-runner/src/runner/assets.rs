use std::fs;
use std::path::PathBuf;

use volt_core::embed::AssetBundle;

use super::RunnerError;
use super::overrides::read_override_bytes_from_env_keys;

pub(super) const ENV_RUNNER_ASSET_BUNDLE_PATH: &str = "VOLT_RUNNER_ASSET_BUNDLE_PATH";
pub(super) const ENV_RUNNER_BACKEND_BUNDLE_PATH: &str = "VOLT_RUNNER_BACKEND_BUNDLE_PATH";
const ENV_RUNNER_ASSET_BUNDLE_LEGACY: &str = "VOLT_ASSET_BUNDLE";
const ENV_RUNNER_BACKEND_BUNDLE_LEGACY: &str = "VOLT_BACKEND_BUNDLE";
const EMBEDDED_ASSET_BUNDLE_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/embedded-assets.bin"));
const EMBEDDED_BACKEND_BUNDLE_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/embedded-backend.js"));

/// Sidecar file names — when a pre-built runner binary is used, the CLI
/// places these files alongside the exe instead of embedding them.
const SIDECAR_ASSET_BUNDLE: &str = "volt-assets.bin";
const SIDECAR_BACKEND_BUNDLE: &str = "volt-backend.js";

pub(crate) fn load_asset_bundle() -> Result<AssetBundle, RunnerError> {
    // 1. Env var override (dev/testing)
    if let Some(bytes) = read_override_bytes_from_env_keys(&[
        ENV_RUNNER_ASSET_BUNDLE_PATH,
        ENV_RUNNER_ASSET_BUNDLE_LEGACY,
    ])? {
        return load_asset_bundle_from_bytes(&bytes);
    }
    // 2. Sidecar file alongside the exe (pre-built runner)
    if let Some(bytes) = read_sidecar_file(SIDECAR_ASSET_BUNDLE) {
        return load_asset_bundle_from_bytes(&bytes);
    }
    // 3. Embedded bytes (compiled-in)
    load_asset_bundle_from_bytes(EMBEDDED_ASSET_BUNDLE_BYTES)
}

pub(crate) fn load_backend_bundle_source() -> Result<String, RunnerError> {
    // 1. Env var override (dev/testing)
    if let Some(bytes) = read_override_bytes_from_env_keys(&[
        ENV_RUNNER_BACKEND_BUNDLE_PATH,
        ENV_RUNNER_BACKEND_BUNDLE_LEGACY,
    ])? {
        return decode_backend_bundle_bytes(
            &bytes,
            format!("{ENV_RUNNER_BACKEND_BUNDLE_PATH} override"),
        );
    }
    // 2. Sidecar file alongside the exe (pre-built runner)
    if let Some(bytes) = read_sidecar_file(SIDECAR_BACKEND_BUNDLE) {
        return decode_backend_bundle_bytes(&bytes, "sidecar backend bundle".to_string());
    }
    // 3. Embedded bytes (compiled-in)
    decode_backend_bundle_bytes(
        EMBEDDED_BACKEND_BUNDLE_BYTES,
        "embedded backend bundle".to_string(),
    )
}

/// Try to read a sidecar file located alongside the current executable.
fn read_sidecar_file(name: &str) -> Option<Vec<u8>> {
    let exe_dir = exe_directory()?;
    let path = exe_dir.join(name);
    fs::read(&path).ok()
}

/// Resolve the directory containing the current executable, following symlinks.
fn exe_directory() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| fs::canonicalize(p).ok())
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub(super) fn load_asset_bundle_from_bytes(bytes: &[u8]) -> Result<AssetBundle, RunnerError> {
    AssetBundle::from_bytes(bytes).map_err(RunnerError::AssetBundle)
}

fn decode_backend_bundle_bytes(bytes: &[u8], source_name: String) -> Result<String, RunnerError> {
    String::from_utf8(bytes.to_vec()).map_err(|error| {
        RunnerError::Config(format!(
            "{source_name} is not valid UTF-8 JavaScript: {}",
            error.utf8_error()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_assets_load_successfully() {
        let bundle =
            load_asset_bundle_from_bytes(EMBEDDED_ASSET_BUNDLE_BYTES).expect("embedded assets");
        assert!(bundle.get("index.html").is_some());
        assert!(bundle.get("assets/main.js").is_some());
        assert!(bundle.get("assets/styles.css").is_some());
    }

    #[test]
    fn invalid_asset_bundle_payload_is_rejected() {
        let err = load_asset_bundle_from_bytes(&[1, 2, 3]).expect_err("invalid payload");
        assert!(matches!(err, RunnerError::AssetBundle(_)));
    }
}
