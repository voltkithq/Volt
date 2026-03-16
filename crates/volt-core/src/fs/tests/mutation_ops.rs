use super::*;
use std::env;
use std::fs;

#[test]
fn test_write_and_read() {
    let base = env::temp_dir();
    let test_dir = "volt_test_fs";
    let test_file = &format!("{test_dir}/test.txt");

    write_file(&base, test_file, b"hello volt").unwrap();

    let content = read_file_text(&base, test_file).unwrap();
    assert_eq!(content, "hello volt");

    let info = stat(&base, test_file).unwrap();
    assert!(info.is_file);
    assert_eq!(info.size, 10);

    let entries = read_dir(&base, test_dir).unwrap();
    assert!(entries.contains(&"test.txt".to_string()));

    remove(&base, test_dir).unwrap();
}

#[test]
fn test_stat_directory() {
    let base = env::temp_dir();
    let dir_name = "volt_test_stat_dir";
    mkdir(&base, dir_name).unwrap();

    let info = stat(&base, dir_name).unwrap();
    assert!(info.is_dir);
    assert!(!info.is_file);

    remove(&base, dir_name).unwrap();
}

#[test]
fn test_mkdir_nested() {
    let base = env::temp_dir();
    let nested = "volt_test_nested/a/b/c";
    mkdir(&base, nested).unwrap();

    let info = stat(&base, nested).unwrap();
    assert!(info.is_dir);

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

    remove(&base, "volt_test_auto_parent").unwrap();
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
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    assert!(info.created_ms.is_some(), "created_ms should be available");

    remove(&base, file).unwrap();
}

#[test]
fn test_exists_returns_true_for_existing_file() {
    let base = env::temp_dir();
    let file = "volt_test_exists_true.txt";
    write_file(&base, file, b"exists").unwrap();

    assert!(exists(&base, file).unwrap());

    remove(&base, file).unwrap();
}

#[test]
fn test_exists_returns_false_for_missing_file() {
    let base = env::temp_dir();
    assert!(!exists(&base, "volt_test_exists_missing_12345.txt").unwrap());
}
