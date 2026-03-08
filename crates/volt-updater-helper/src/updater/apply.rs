use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

pub(crate) fn replace_binary_with_retries(
    target_path: &Path,
    staged_path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    let mut last_error = String::new();
    for attempt in 1..=30 {
        match try_replace_binary(target_path, staged_path, expected_sha256) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = error;
                log_warn(&format!(
                    "replace attempt {attempt}/30 failed: {last_error}"
                ));
                thread::sleep(Duration::from_millis(250));
            }
        }
    }

    Err(format!(
        "failed to replace binary after retries: {last_error}"
    ))
}

pub(crate) fn rollback_from_backup(
    target_path: &Path,
    backup_path: &Path,
    expected_backup_sha256: &str,
    pending_marker_path: &Path,
) -> Result<(), String> {
    let backup_hash = sha256_file(backup_path).map_err(|error| {
        format!(
            "failed to hash rollback backup '{}': {error}",
            backup_path.display()
        )
    })?;
    if backup_hash != expected_backup_sha256 {
        return Err(format!(
            "rollback backup checksum mismatch: expected {expected_backup_sha256}, got {backup_hash}"
        ));
    }

    let failed_path = target_path.with_extension("failed");
    remove_file_if_exists(&failed_path, "stale failed-update payload")?;
    let target_existed = target_path.exists();
    if target_existed {
        std::fs::rename(target_path, &failed_path).map_err(|error| {
            format!(
                "failed to move failed target '{}' aside before rollback: {error}",
                target_path.display()
            )
        })?;
    }

    if let Err(error) = std::fs::rename(backup_path, target_path) {
        if target_existed {
            let _ = std::fs::rename(&failed_path, target_path);
        }
        return Err(format!(
            "failed to restore rollback backup '{}' to '{}': {error}",
            backup_path.display(),
            target_path.display()
        ));
    }

    let restored_hash = sha256_file(target_path).map_err(|error| {
        format!(
            "failed to hash restored target '{}' after rollback: {error}",
            target_path.display()
        )
    })?;
    if restored_hash != expected_backup_sha256 {
        return Err(format!(
            "restored target checksum mismatch after rollback: expected {expected_backup_sha256}, got {restored_hash}"
        ));
    }

    remove_file_with_warning(&failed_path, "failed-update payload");
    remove_file_with_warning(pending_marker_path, "pending-update marker");
    Ok(())
}

fn try_replace_binary(
    target_path: &Path,
    staged_path: &Path,
    expected_sha256: &str,
) -> Result<(), String> {
    let mut staged_file = File::open(staged_path).map_err(|error| {
        format!(
            "failed to open staged payload '{}': {error}",
            staged_path.display()
        )
    })?;
    let staged_hash = sha256_reader(&mut staged_file).map_err(|error| {
        format!(
            "failed to hash staged payload '{}': {error}",
            staged_path.display()
        )
    })?;
    if staged_hash != expected_sha256 {
        return Err(format!(
            "staged payload checksum mismatch: expected {expected_sha256}, got {staged_hash}"
        ));
    }
    staged_file
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("failed to rewind staged payload: {error}"))?;

    let backup_path = target_path.with_extension("old");
    remove_file_if_exists(&backup_path, "stale backup")?;

    let target_existed = target_path.exists();
    if target_existed {
        std::fs::rename(target_path, &backup_path).map_err(|error| {
            format!(
                "failed to rename target '{}' to backup '{}': {error}",
                target_path.display(),
                backup_path.display()
            )
        })?;
    }

    let copy_result = (|| -> Result<(), String> {
        let mut target_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target_path)
            .map_err(|error| {
                format!(
                    "failed to create target executable '{}': {error}",
                    target_path.display()
                )
            })?;
        std::io::copy(&mut staged_file, &mut target_file).map_err(|error| {
            format!(
                "failed to copy staged payload '{}' to target '{}': {error}",
                staged_path.display(),
                target_path.display()
            )
        })?;
        target_file
            .flush()
            .map_err(|error| format!("failed to flush target executable: {error}"))?;
        target_file
            .sync_all()
            .map_err(|error| format!("failed to sync target executable: {error}"))?;
        drop(target_file);

        let target_hash = sha256_file(target_path).map_err(|error| {
            format!(
                "failed to hash copied target executable '{}': {error}",
                target_path.display()
            )
        })?;
        if target_hash != expected_sha256 {
            return Err(format!(
                "target executable checksum mismatch after copy: expected {expected_sha256}, got {target_hash}"
            ));
        }

        Ok(())
    })();

    match copy_result {
        Ok(()) => {
            remove_file_with_warning(staged_path, "staged payload");
            if target_existed {
                // Keep the backup payload until first-launch health checks complete.
                log_info("retaining backup payload for startup rollback window");
            }
            Ok(())
        }
        Err(error) => {
            remove_file_with_warning(target_path, "failed replacement target");
            if target_existed {
                std::fs::rename(&backup_path, target_path).map_err(|restore_error| {
                    format!(
                        "{error}; rollback failed restoring '{}' from '{}': {restore_error}",
                        target_path.display(),
                        backup_path.display()
                    )
                })?;
            }
            Err(error)
        }
    }
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    sha256_reader(&mut file)
}

fn sha256_reader(reader: &mut impl Read) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn remove_file_if_exists(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "failed to remove {label} '{}': {error}",
            path.display()
        )),
    }
}

fn remove_file_with_warning(path: &Path, label: &str) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log_warn(&format!(
            "failed to remove {label} '{}': {error}",
            path.display()
        ));
    }
}

pub(crate) fn log_info(message: &str) {
    tracing::info!(message = message, "updater helper");
}

fn log_warn(message: &str) {
    tracing::warn!(message = message, "updater helper warning");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_from_backup_restores_previous_binary_and_clears_marker() {
        let temp_root = std::env::temp_dir().join(format!(
            "volt-updater-helper-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp root");

        let target_path = temp_root.join("app.exe");
        let backup_path = temp_root.join("app.old");
        let marker_path = temp_root.join("app.exe.volt-update-pending.json");
        std::fs::write(&target_path, b"broken-new-binary").expect("write target");
        std::fs::write(&backup_path, b"known-good-binary").expect("write backup");
        std::fs::write(&marker_path, b"{\"ok\":true}").expect("write marker");

        let backup_sha = sha256_file(&backup_path).expect("hash backup");
        rollback_from_backup(&target_path, &backup_path, &backup_sha, &marker_path)
            .expect("rollback succeeds");

        let restored = std::fs::read(&target_path).expect("read restored target");
        assert_eq!(restored, b"known-good-binary");
        assert!(!backup_path.exists());
        assert!(!marker_path.exists());

        let _ = std::fs::remove_file(target_path);
        let _ = std::fs::remove_dir_all(temp_root);
    }
}
