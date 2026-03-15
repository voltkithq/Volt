use super::*;
use std::env;

#[test]
fn test_path_traversal_rejected() {
    let base = env::temp_dir();
    assert!(safe_resolve(&base, "../../etc/passwd").is_err());
    assert!(safe_resolve(&base, "/etc/passwd").is_err());
}

#[test]
fn test_valid_relative_path() {
    let base = env::temp_dir();
    let result = safe_resolve(&base, "test_volt_file.txt");
    assert!(result.is_ok());
    assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
}

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
    let security = FsError::Security("bad path".into());
    assert!(security.to_string().contains("bad path"));

    let out_of_scope = FsError::OutOfScope;
    assert!(out_of_scope.to_string().contains("outside"));
}

#[test]
fn test_safe_resolve_allows_double_dot_inside_component() {
    let base = env::temp_dir();
    let result = safe_resolve(&base, "volt_test_a..b/file.txt");
    assert!(result.is_ok());
    assert!(result.unwrap().starts_with(base.canonicalize().unwrap()));
}

#[test]
fn test_exists_rejects_traversal() {
    let base = env::temp_dir();
    assert!(exists(&base, "../../etc/passwd").is_err());
}
