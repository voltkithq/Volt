use super::*;
use std::env;
use std::path::Path;

#[cfg(unix)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[test]
fn test_path_traversal_rejected() {
    let base = env::temp_dir();
    assert!(safe_resolve(&base, "../../etc/passwd").is_err());
    assert!(safe_resolve(&base, "/etc/passwd").is_err());
}

#[test]
fn test_valid_relative_path() {
    let base = env::temp_dir();
    // This should succeed since temp dir exists
    let result = safe_resolve(&base, "test_volt_file.txt");
    assert!(result.is_ok());
    assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
}

#[test]
fn test_write_and_read() {
    let base = env::temp_dir();
    let test_dir = "volt_test_fs";
    let test_file = &format!("{test_dir}/test.txt");

    // Write
    write_file(&base, test_file, b"hello volt").unwrap();

    // Read
    let content = read_file_text(&base, test_file).unwrap();
    assert_eq!(content, "hello volt");

    // Stat
    let info = stat(&base, test_file).unwrap();
    assert!(info.is_file);
    assert_eq!(info.size, 10);

    // Read dir
    let entries = read_dir(&base, test_dir).unwrap();
    assert!(entries.contains(&"test.txt".to_string()));

    // Clean up
    remove(&base, test_dir).unwrap();
}

// ── Expanded tests ─────────────────────────────────────────────

#[test]
fn test_backslash_path_rejected() {
    let base = env::temp_dir();
    assert!(safe_resolve(&base, "\\etc\\passwd").is_err());
}

#[test]
fn test_windows_drive_letter_rejected() {
    let base = env::temp_dir();
    assert!(safe_resolve(&base, "C:\\Windows\\System32").is_err());
    assert!(safe_resolve(&base, "D:\\data.txt").is_err());
}

#[test]
fn test_stat_directory() {
    let base = env::temp_dir();
    let dir_name = "volt_test_stat_dir";
    mkdir(&base, dir_name).unwrap();

    let info = stat(&base, dir_name).unwrap();
    assert!(info.is_dir);
    assert!(!info.is_file);

    // Clean up
    remove(&base, dir_name).unwrap();
}

#[test]
fn test_mkdir_nested() {
    let base = env::temp_dir();
    let nested = "volt_test_nested/a/b/c";
    mkdir(&base, nested).unwrap();

    let info = stat(&base, nested).unwrap();
    assert!(info.is_dir);

    // Clean up
    remove(&base, "volt_test_nested").unwrap();
}

#[test]
fn test_remove_file() {
    let base = env::temp_dir();
    let file = "volt_test_remove_file.txt";
    write_file(&base, file, b"to be removed").unwrap();

    let resolved = safe_resolve(&base, file).unwrap();
    assert!(resolved.exists());

    remove(&base, file).unwrap();
    assert!(!resolved.exists());
}

#[test]
fn test_read_dir_empty() {
    let base = env::temp_dir();
    let dir_name = "volt_test_empty_dir";
    mkdir(&base, dir_name).unwrap();

    let entries = read_dir(&base, dir_name).unwrap();
    assert!(entries.is_empty());

    // Clean up
    remove(&base, dir_name).unwrap();
}

#[test]
fn test_read_nonexistent_file_error() {
    let base = env::temp_dir();
    let result = read_file(&base, "volt_definitely_does_not_exist_12345.txt");
    assert!(result.is_err());
}

#[test]
fn test_write_creates_parent_dirs() {
    let base = env::temp_dir();
    let path = "volt_test_auto_parent/sub1/sub2/file.txt";
    write_file(&base, path, b"deep file").unwrap();

    let content = read_file_text(&base, path).unwrap();
    assert_eq!(content, "deep file");

    // Clean up
    remove(&base, "volt_test_auto_parent").unwrap();
}

#[test]
fn test_safe_resolve_for_create_rejects_symlinked_parent_escape() {
    let base = env::temp_dir().join("volt_test_create_scope_guard");
    let outside = env::temp_dir().join("volt_test_create_scope_guard_outside");
    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&outside).unwrap();

    if create_dir_symlink(&outside, &base.join("linked")).is_err() {
        let _ = fs::remove_dir_all(&base);
        let _ = fs::remove_dir_all(&outside);
        return;
    }

    let result = safe_resolve_for_create(&base, "linked/escape.txt");
    assert!(matches!(
        result,
        Err(FsError::Security(_)) | Err(FsError::OutOfScope)
    ));

    let _ = fs::remove_dir_all(&base);
    let _ = fs::remove_dir_all(&outside);
}

#[test]
fn test_fs_error_display() {
    let e = FsError::Security("bad path".into());
    assert!(e.to_string().contains("bad path"));

    let e = FsError::OutOfScope;
    assert!(e.to_string().contains("outside"));
}

#[test]
fn test_safe_resolve_allows_double_dot_inside_component() {
    let base = env::temp_dir();
    let result = safe_resolve(&base, "volt_test_a..b/file.txt");
    assert!(result.is_ok());
    assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
}

#[test]
fn test_remove_rejects_base_directory_targets() {
    let base = env::temp_dir().join("volt_test_remove_base_guard");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("keep.txt"), b"keep").unwrap();

    assert!(remove(&base, ".").is_err());
    assert!(remove(&base, "").is_err());
    assert!(base.exists());

    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_stat_returns_timestamps() {
    let base = env::temp_dir();
    let file = "volt_test_stat_timestamps.txt";
    write_file(&base, file, b"timestamp test").unwrap();

    let info = stat(&base, file).unwrap();
    assert!(info.modified_ms > 0.0, "modified_ms should be positive");
    // created_ms may be None on some Linux filesystems, but should be
    // Some on Windows and macOS.
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    assert!(info.created_ms.is_some(), "created_ms should be available");

    // Clean up
    remove(&base, file).unwrap();
}

#[test]
fn test_exists_returns_true_for_existing_file() {
    let base = env::temp_dir();
    let file = "volt_test_exists_true.txt";
    write_file(&base, file, b"exists").unwrap();

    assert!(exists(&base, file).unwrap());

    // Clean up
    remove(&base, file).unwrap();
}

#[test]
fn test_exists_returns_false_for_missing_file() {
    let base = env::temp_dir();
    assert!(!exists(&base, "volt_test_exists_missing_12345.txt").unwrap());
}

#[test]
fn test_exists_rejects_traversal() {
    let base = env::temp_dir();
    assert!(exists(&base, "../../etc/passwd").is_err());
}

#[test]
fn test_rename_file() {
    let base = env::temp_dir();
    let from = "volt_test_rename_from.txt";
    let to = "volt_test_rename_to.txt";
    write_file(&base, from, b"rename me").unwrap();

    rename(&base, from, to).unwrap();

    assert!(!exists(&base, from).unwrap());
    assert!(exists(&base, to).unwrap());
    let content = read_file_text(&base, to).unwrap();
    assert_eq!(content, "rename me");

    remove(&base, to).unwrap();
}

#[test]
fn test_rename_rejects_missing_source() {
    let base = env::temp_dir();
    let result = rename(
        &base,
        "volt_test_rename_missing.txt",
        "volt_test_rename_dest.txt",
    );
    assert!(result.is_err());
}

#[test]
fn test_rename_rejects_existing_destination() {
    let base = env::temp_dir();
    let from = "volt_test_rename_conflict_from.txt";
    let to = "volt_test_rename_conflict_to.txt";
    write_file(&base, from, b"source").unwrap();
    write_file(&base, to, b"dest").unwrap();

    let result = rename(&base, from, to);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("FS_ALREADY_EXISTS")
    );

    remove(&base, from).unwrap();
    remove(&base, to).unwrap();
}

#[test]
fn test_rename_directory() {
    let base = env::temp_dir();
    let from_dir = "volt_test_rename_dir_from";
    let to_dir = "volt_test_rename_dir_to";
    mkdir(&base, from_dir).unwrap();
    write_file(&base, &format!("{from_dir}/file.txt"), b"inside").unwrap();

    rename(&base, from_dir, to_dir).unwrap();

    assert!(!exists(&base, from_dir).unwrap());
    assert!(exists(&base, to_dir).unwrap());
    let content = read_file_text(&base, &format!("{to_dir}/file.txt")).unwrap();
    assert_eq!(content, "inside");

    remove(&base, to_dir).unwrap();
}

#[test]
fn test_copy_file() {
    let base = env::temp_dir();
    let from = "volt_test_copy_from.txt";
    let to = "volt_test_copy_to.txt";
    write_file(&base, from, b"copy me").unwrap();

    copy(&base, from, to).unwrap();

    assert!(exists(&base, from).unwrap());
    assert!(exists(&base, to).unwrap());
    let content = read_file_text(&base, to).unwrap();
    assert_eq!(content, "copy me");

    remove(&base, from).unwrap();
    remove(&base, to).unwrap();
}

#[test]
fn test_copy_rejects_missing_source() {
    let base = env::temp_dir();
    let result = copy(
        &base,
        "volt_test_copy_missing.txt",
        "volt_test_copy_dest.txt",
    );
    assert!(result.is_err());
}

#[test]
fn test_copy_rejects_directory() {
    let base = env::temp_dir();
    let dir = "volt_test_copy_dir_reject";
    mkdir(&base, dir).unwrap();

    let result = copy(&base, dir, "volt_test_copy_dir_dest");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("files, not directories")
    );

    remove(&base, dir).unwrap();
}

#[test]
fn test_copy_rejects_existing_destination() {
    let base = env::temp_dir();
    let from = "volt_test_copy_conflict_from.txt";
    let to = "volt_test_copy_conflict_to.txt";
    write_file(&base, from, b"source").unwrap();
    write_file(&base, to, b"dest").unwrap();

    let result = copy(&base, from, to);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("FS_ALREADY_EXISTS")
    );

    remove(&base, from).unwrap();
    remove(&base, to).unwrap();
}

#[test]
fn test_rename_rejects_traversal() {
    let base = env::temp_dir();
    let from = "volt_test_rename_trav.txt";
    write_file(&base, from, b"data").unwrap();

    assert!(rename(&base, from, "../../etc/evil.txt").is_err());
    assert!(rename(&base, "../../etc/passwd", "stolen.txt").is_err());

    remove(&base, from).unwrap();
}

#[test]
fn test_copy_rejects_traversal() {
    let base = env::temp_dir();
    let from = "volt_test_copy_trav.txt";
    write_file(&base, from, b"data").unwrap();

    assert!(copy(&base, from, "../../etc/evil.txt").is_err());
    assert!(copy(&base, "../../etc/passwd", "stolen.txt").is_err());

    remove(&base, from).unwrap();
}
