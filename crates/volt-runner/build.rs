use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_ASSET_BUNDLE: &str = "VOLT_ASSET_BUNDLE";
const ENV_BACKEND_BUNDLE: &str = "VOLT_BACKEND_BUNDLE";
const ENV_RUNNER_CONFIG: &str = "VOLT_RUNNER_CONFIG";
const ENV_UPDATE_PUBLIC_KEY: &str = "VOLT_UPDATE_PUBLIC_KEY";
const ENV_APP_ICON: &str = "VOLT_APP_ICON";
const ENV_APP_NAME: &str = "VOLT_APP_NAME";
const ENV_APP_VERSION: &str = "VOLT_APP_VERSION";
const DEFAULT_ASSET_BUNDLE: &str = "assets/default-assets.bin";
const DEFAULT_BACKEND_BUNDLE: &str = "assets/default-backend.js";
const DEFAULT_RUNNER_CONFIG: &str = "assets/default-config.json";
const OUT_ASSET_BUNDLE: &str = "embedded-assets.bin";
const OUT_BACKEND_BUNDLE: &str = "embedded-backend.js";
const OUT_RUNNER_CONFIG: &str = "embedded-config.json";
const OUT_UPDATE_PUBLIC_KEY: &str = "embedded-update-public-key.txt";

// Sentinel markers for pre-built shell binaries.
// These unique byte sequences are used to locate placeholder slots in the binary
// so they can be patched with real app data without recompilation.
const SENTINEL_ASSET_BUNDLE: &[u8; 32] = b"__VOLT_SENTINEL_ASSET_BUNDLE_V1_";
const SENTINEL_BACKEND_BUNDLE: &[u8; 32] = b"__VOLT_SENTINEL_BACKEND_BNDL_V1_";
const SENTINEL_RUNNER_CONFIG: &[u8; 32] = b"__VOLT_SENTINEL_RUNNER_CONFG_V1_";

// Maximum slot sizes for pre-built shell placeholders.
// These define how large the embedded data can be when patching.
const SHELL_MAX_ASSET_BUNDLE: usize = 64 * 1024 * 1024; // 64 MB
const SHELL_MAX_BACKEND_BUNDLE: usize = 4 * 1024 * 1024; // 4 MB
const SHELL_MAX_RUNNER_CONFIG: usize = 256 * 1024; // 256 KB

fn main() {
    println!("cargo:rerun-if-env-changed={ENV_ASSET_BUNDLE}");
    println!("cargo:rerun-if-env-changed={ENV_BACKEND_BUNDLE}");
    println!("cargo:rerun-if-env-changed={ENV_RUNNER_CONFIG}");
    println!("cargo:rerun-if-env-changed={ENV_UPDATE_PUBLIC_KEY}");
    println!("cargo:rerun-if-env-changed={ENV_APP_ICON}");
    println!("cargo:rerun-if-env-changed={ENV_APP_NAME}");
    println!("cargo:rerun-if-env-changed={ENV_APP_VERSION}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not set by cargo"));
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set by cargo"),
    );

    if cfg!(feature = "prebuilt-shell") {
        // Generate sentinel-marked placeholder slots for binary patching.
        write_sentinel_placeholder(&out_dir.join(OUT_ASSET_BUNDLE), SENTINEL_ASSET_BUNDLE, SHELL_MAX_ASSET_BUNDLE)
            .unwrap_or_else(|e| panic!("failed to write asset bundle placeholder: {e}"));
        write_sentinel_placeholder(&out_dir.join(OUT_BACKEND_BUNDLE), SENTINEL_BACKEND_BUNDLE, SHELL_MAX_BACKEND_BUNDLE)
            .unwrap_or_else(|e| panic!("failed to write backend bundle placeholder: {e}"));
        write_sentinel_placeholder(&out_dir.join(OUT_RUNNER_CONFIG), SENTINEL_RUNNER_CONFIG, SHELL_MAX_RUNNER_CONFIG)
            .unwrap_or_else(|e| panic!("failed to write runner config placeholder: {e}"));
    } else {
        embed_artifact(
            ENV_ASSET_BUNDLE,
            &manifest_dir.join(DEFAULT_ASSET_BUNDLE),
            &out_dir.join(OUT_ASSET_BUNDLE),
        )
        .unwrap_or_else(|error| panic!("failed to embed frontend asset bundle: {error}"));

        embed_artifact(
            ENV_BACKEND_BUNDLE,
            &manifest_dir.join(DEFAULT_BACKEND_BUNDLE),
            &out_dir.join(OUT_BACKEND_BUNDLE),
        )
        .unwrap_or_else(|error| panic!("failed to embed backend bundle: {error}"));

        embed_artifact(
            ENV_RUNNER_CONFIG,
            &manifest_dir.join(DEFAULT_RUNNER_CONFIG),
            &out_dir.join(OUT_RUNNER_CONFIG),
        )
        .unwrap_or_else(|error| panic!("failed to embed runner config: {error}"));
    }

    write_embedded_update_public_key(&out_dir.join(OUT_UPDATE_PUBLIC_KEY))
        .unwrap_or_else(|error| panic!("failed to embed update public key: {error}"));

    embed_windows_resource();
}

#[cfg(windows)]
fn embed_windows_resource() {
    let mut res = winresource::WindowsResource::new();

    if let Ok(icon_path) = env::var(ENV_APP_ICON) {
        let icon = strip_surrounding_quotes(icon_path.trim()).trim().to_string();
        if !icon.is_empty() && Path::new(&icon).exists() {
            println!("cargo:rerun-if-changed={icon}");
            res.set_icon(&icon);
        }
    }

    if let Ok(name) = env::var(ENV_APP_NAME) {
        let name = strip_surrounding_quotes(name.trim()).trim().to_string();
        if !name.is_empty() {
            res.set("ProductName", &name);
            res.set("FileDescription", &name);
        }
    }

    if let Ok(version) = env::var(ENV_APP_VERSION) {
        let version = strip_surrounding_quotes(version.trim()).trim().to_string();
        if !version.is_empty() {
            res.set("FileVersion", &version);
            res.set("ProductVersion", &version);
        }
    }

    res.compile().expect("failed to compile Windows resources");
}

#[cfg(not(windows))]
fn embed_windows_resource() {
    // No-op on non-Windows platforms.
}

fn embed_artifact(env_key: &str, default_path: &Path, out_path: &Path) -> Result<(), String> {
    let source_path = match env::var(env_key) {
        Ok(raw) => resolve_env_source_path(&raw)?,
        Err(env::VarError::NotPresent) => default_path.to_path_buf(),
        Err(env::VarError::NotUnicode(_)) => {
            return Err(format!("{env_key} contains non-Unicode data"));
        }
    };

    if !source_path.exists() {
        return Err(format!(
            "source file does not exist: {}",
            source_path.display()
        ));
    }

    println!("cargo:rerun-if-changed={}", source_path.display());
    fs::copy(&source_path, out_path).map_err(|error| {
        format!(
            "failed to copy '{}' to '{}': {error}",
            source_path.display(),
            out_path.display()
        )
    })?;
    Ok(())
}

fn resolve_env_source_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = strip_surrounding_quotes(raw.trim());
    if trimmed.is_empty() {
        return Err("path override is empty".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        return Ok(candidate);
    }

    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let cwd_candidate = cwd.join(&candidate);
    if cwd_candidate.exists() {
        return Ok(cwd_candidate);
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(|error| format!("failed to read CARGO_MANIFEST_DIR: {error}"))?;
    Ok(manifest_dir.join(candidate))
}

fn strip_surrounding_quotes(input: &str) -> &str {
    if input.len() >= 2 {
        let bytes = input.as_bytes();
        let first = bytes[0];
        let last = bytes[input.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &input[1..input.len() - 1];
        }
    }
    input
}

/// Write a sentinel-marked placeholder file for binary patching.
/// Layout: [32-byte sentinel] [4-byte LE actual_length = 0] [zero-padding to max_size]
/// Total file size: 32 + 4 + max_size
fn write_sentinel_placeholder(out_path: &Path, sentinel: &[u8; 32], max_size: usize) -> Result<(), String> {
    let total = 32 + 4 + max_size;
    let mut data = vec![0u8; total];
    data[..32].copy_from_slice(sentinel);
    // actual_length = 0 (LE u32) at bytes 32..36 — already zero
    fs::write(out_path, &data).map_err(|e| {
        format!("failed to write sentinel placeholder to '{}': {e}", out_path.display())
    })
}

fn write_embedded_update_public_key(out_path: &Path) -> Result<(), String> {
    let key = match env::var(ENV_UPDATE_PUBLIC_KEY) {
        Ok(value) => strip_surrounding_quotes(value.trim()).trim().to_string(),
        Err(env::VarError::NotPresent) => String::new(),
        Err(env::VarError::NotUnicode(_)) => {
            return Err(format!("{ENV_UPDATE_PUBLIC_KEY} contains non-Unicode data"));
        }
    };

    fs::write(out_path, key).map_err(|error| {
        format!(
            "failed to write embedded update public key to '{}': {error}",
            out_path.display()
        )
    })
}
