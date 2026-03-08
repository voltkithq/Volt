use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdateMode {
    Install,
    Rollback,
}

#[derive(Debug)]
pub(crate) enum Args {
    Install(InstallArgs),
    Rollback(RollbackArgs),
}

#[derive(Debug)]
pub(crate) struct InstallArgs {
    pub(crate) pid: u32,
    pub(crate) target_path: PathBuf,
    pub(crate) staged_path: PathBuf,
    pub(crate) expected_sha256: String,
    pub(crate) wait_timeout_secs: u64,
}

#[derive(Debug)]
pub(crate) struct RollbackArgs {
    pub(crate) pid: u32,
    pub(crate) target_path: PathBuf,
    pub(crate) backup_path: PathBuf,
    pub(crate) backup_sha256: String,
    pub(crate) pending_marker_path: PathBuf,
    pub(crate) wait_timeout_secs: u64,
}

pub(crate) fn parse_args() -> Result<Args, String> {
    parse_args_from_iter(std::env::args().skip(1))
}

pub(crate) fn parse_args_from_iter(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut mode = UpdateMode::Install;
    let mut pid: Option<u32> = None;
    let mut target_path: Option<PathBuf> = None;
    let mut staged_path: Option<PathBuf> = None;
    let mut expected_sha256: Option<String> = None;
    let mut backup_path: Option<PathBuf> = None;
    let mut backup_sha256: Option<String> = None;
    let mut pending_marker_path: Option<PathBuf> = None;
    let mut wait_timeout_secs: u64 = 600;

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--mode" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --mode".to_string())?;
                mode = parse_mode(&value)?;
            }
            "--pid" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --pid".to_string())?;
                pid = Some(
                    value
                        .parse::<u32>()
                        .map_err(|error| format!("invalid --pid value '{value}': {error}"))?,
                );
            }
            "--target" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --target".to_string())?;
                target_path = Some(PathBuf::from(value));
            }
            "--staged" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --staged".to_string())?;
                staged_path = Some(PathBuf::from(value));
            }
            "--sha256" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --sha256".to_string())?;
                expected_sha256 = Some(normalize_sha256_hex(&value, "--sha256")?);
            }
            "--backup" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --backup".to_string())?;
                backup_path = Some(PathBuf::from(value));
            }
            "--backup-sha256" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --backup-sha256".to_string())?;
                backup_sha256 = Some(normalize_sha256_hex(&value, "--backup-sha256")?);
            }
            "--pending-marker" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --pending-marker".to_string())?;
                pending_marker_path = Some(PathBuf::from(value));
            }
            "--wait-timeout-secs" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --wait-timeout-secs".to_string())?;
                wait_timeout_secs = value.parse::<u64>().map_err(|error| {
                    format!("invalid --wait-timeout-secs value '{value}': {error}")
                })?;
            }
            other => {
                return Err(format!("unexpected flag '{other}'"));
            }
        }
    }

    let pid = pid.ok_or_else(|| "missing required --pid".to_string())?;
    let target_path = target_path.ok_or_else(|| "missing required --target".to_string())?;

    if wait_timeout_secs == 0 {
        return Err("--wait-timeout-secs must be greater than zero".to_string());
    }
    if target_path.as_os_str().is_empty() {
        return Err("--target must not be empty".to_string());
    }

    match mode {
        UpdateMode::Install => {
            let staged_path = staged_path.ok_or_else(|| "missing required --staged".to_string())?;
            let expected_sha256 =
                expected_sha256.ok_or_else(|| "missing required --sha256".to_string())?;
            if staged_path.as_os_str().is_empty() {
                return Err("--staged must not be empty".to_string());
            }

            Ok(Args::Install(InstallArgs {
                pid,
                target_path,
                staged_path,
                expected_sha256,
                wait_timeout_secs,
            }))
        }
        UpdateMode::Rollback => {
            let backup_path = backup_path.ok_or_else(|| "missing required --backup".to_string())?;
            let backup_sha256 =
                backup_sha256.ok_or_else(|| "missing required --backup-sha256".to_string())?;
            let pending_marker_path = pending_marker_path
                .ok_or_else(|| "missing required --pending-marker".to_string())?;
            if backup_path.as_os_str().is_empty() {
                return Err("--backup must not be empty".to_string());
            }
            if pending_marker_path.as_os_str().is_empty() {
                return Err("--pending-marker must not be empty".to_string());
            }

            Ok(Args::Rollback(RollbackArgs {
                pid,
                target_path,
                backup_path,
                backup_sha256,
                pending_marker_path,
                wait_timeout_secs,
            }))
        }
    }
}

fn parse_mode(input: &str) -> Result<UpdateMode, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "install" => Ok(UpdateMode::Install),
        "rollback" => Ok(UpdateMode::Rollback),
        other => Err(format!(
            "invalid --mode value '{other}', expected one of: install, rollback"
        )),
    }
}

fn normalize_sha256_hex(input: &str, flag: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.len() != 64 {
        return Err(format!(
            "{flag} must be a 64-character lowercase hex digest"
        ));
    }
    if !trimmed
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{flag} must contain only lowercase hex characters"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_install_args_requires_mandatory_fields() {
        let parsed = parse_args_from(
            [
                "volt-updater-helper",
                "--pid",
                "42",
                "--target",
                "C:\\demo\\app.exe",
                "--staged",
                "C:\\demo\\app.update",
                "--sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ]
            .iter()
            .map(ToString::to_string),
        )
        .expect("parse args");

        let install = match parsed {
            Args::Install(args) => args,
            Args::Rollback(_) => panic!("expected install args"),
        };

        assert_eq!(install.pid, 42);
        assert_eq!(install.wait_timeout_secs, 600);
    }

    #[test]
    fn parse_args_rejects_invalid_sha256() {
        let result = parse_args_from(
            [
                "volt-updater-helper",
                "--pid",
                "42",
                "--target",
                "C:\\demo\\app.exe",
                "--staged",
                "C:\\demo\\app.update",
                "--sha256",
                "ABC",
            ]
            .iter()
            .map(ToString::to_string),
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_rollback_args_requires_rollback_fields() {
        let parsed = parse_args_from(
            [
                "volt-updater-helper",
                "--mode",
                "rollback",
                "--pid",
                "77",
                "--target",
                "C:\\demo\\app.exe",
                "--backup",
                "C:\\demo\\app.old",
                "--backup-sha256",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "--pending-marker",
                "C:\\demo\\app.exe.volt-update-pending.json",
            ]
            .iter()
            .map(ToString::to_string),
        )
        .expect("parse rollback args");

        let rollback = match parsed {
            Args::Rollback(args) => args,
            Args::Install(_) => panic!("expected rollback args"),
        };

        assert_eq!(rollback.pid, 77);
        assert_eq!(rollback.wait_timeout_secs, 600);
    }

    fn parse_args_from(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
        let mut iter = args.into_iter();
        let _ = iter.next();
        parse_args_from_iter(iter)
    }
}
