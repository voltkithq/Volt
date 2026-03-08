use std::env::{self, VarError};
use std::fs;

use super::RunnerError;

pub(super) fn read_override_bytes_from_env_keys(
    env_keys: &[&str],
) -> Result<Option<Vec<u8>>, RunnerError> {
    for env_key in env_keys {
        if let Some(bytes) = read_override_bytes_from_env(env_key)? {
            return Ok(Some(bytes));
        }
    }
    Ok(None)
}

pub(super) fn read_override_bytes_from_path_value(
    env_key: &str,
    raw_path: &str,
) -> Result<Vec<u8>, RunnerError> {
    let normalized_path = normalize_override_path(env_key, raw_path)?;
    fs::read(&normalized_path).map_err(|source| RunnerError::Io {
        context: format!("failed to read {env_key} path '{normalized_path}'"),
        source,
    })
}

pub(super) fn normalize_override_path(
    env_key: &str,
    raw_path: &str,
) -> Result<String, RunnerError> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(RunnerError::Config(format!("{env_key} is set but empty")));
    }

    let normalized = strip_surrounding_quotes(trimmed).trim();
    if normalized.is_empty() {
        return Err(RunnerError::Config(format!("{env_key} is set but empty")));
    }

    Ok(normalized.to_string())
}

fn read_override_bytes_from_env(env_key: &str) -> Result<Option<Vec<u8>>, RunnerError> {
    match env::var(env_key) {
        Ok(path) => read_override_bytes_from_path_value(env_key, &path).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(RunnerError::Config(format!(
            "{env_key} contains non-Unicode data"
        ))),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_file_path(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "volt-runner-{prefix}-{}-{nonce}.tmp",
            std::process::id()
        ))
    }

    #[test]
    fn quoted_override_path_is_supported() {
        let fixture_path = unique_temp_file_path("quoted-path");
        fs::write(&fixture_path, b"fixture").expect("write fixture bytes");
        let quoted_path = format!("\"{}\"", fixture_path.display());

        let loaded = read_override_bytes_from_path_value("TEST_ENV", &quoted_path)
            .expect("load quoted path value");
        assert_eq!(loaded, b"fixture");

        let _ = fs::remove_file(fixture_path);
    }

    #[test]
    fn empty_override_path_value_is_rejected() {
        let err = read_override_bytes_from_path_value("TEST_ENV", "   ")
            .expect_err("empty path should fail");
        assert!(matches!(err, RunnerError::Config(_)));
    }
}
