//! Integration tests combining fs::safe_resolve with security::validate_path.
//! Verifies the full defense-in-depth chain against path-based attacks.

use std::path::Path;
use volt_core::fs::{mkdir, read_dir, read_file_text, remove, safe_resolve, stat, write_file};

/// Helper to create a temporary test sandbox.
fn create_sandbox() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "volt_security_fs_integration_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn safe_resolve_rejects_all_traversal_variants() {
    let sandbox = create_sandbox();

    // Basic traversal
    assert!(safe_resolve(&sandbox, "../../etc/passwd").is_err());

    // Backslash traversal
    assert!(safe_resolve(&sandbox, "..\\..\\etc\\passwd").is_err());

    // Absolute path (Unix-style)
    assert!(safe_resolve(&sandbox, "/etc/passwd").is_err());

    // Absolute path (Windows drive letter)
    assert!(safe_resolve(&sandbox, "C:\\Windows\\System32\\cmd.exe").is_err());

    // Leading backslash (absolute on Windows)
    assert!(safe_resolve(&sandbox, "\\Windows\\System32").is_err());

    // Reserved device names
    assert!(safe_resolve(&sandbox, "CON").is_err());
    assert!(safe_resolve(&sandbox, "NUL").is_err());
    assert!(safe_resolve(&sandbox, "COM1").is_err());
    assert!(safe_resolve(&sandbox, "con.txt").is_err());

    // But normal paths should work
    assert!(safe_resolve(&sandbox, "data.txt").is_ok());
    assert!(safe_resolve(&sandbox, "subdir/file.json").is_ok());

    cleanup(&sandbox);
}

#[test]
fn full_fs_operations_within_sandbox() {
    let sandbox = create_sandbox();

    // Write a file
    write_file(&sandbox, "hello.txt", b"Hello, Volt!").unwrap();

    // Read it back
    let content = read_file_text(&sandbox, "hello.txt").unwrap();
    assert_eq!(content, "Hello, Volt!");

    // Stat it
    let info = stat(&sandbox, "hello.txt").unwrap();
    assert!(info.is_file);
    assert!(!info.is_dir);
    assert_eq!(info.size, 12);

    // Create nested directories and files
    mkdir(&sandbox, "data/nested/deep").unwrap();
    write_file(
        &sandbox,
        "data/nested/deep/config.json",
        b"{\"key\":\"value\"}",
    )
    .unwrap();

    let config = read_file_text(&sandbox, "data/nested/deep/config.json").unwrap();
    assert_eq!(config, "{\"key\":\"value\"}");

    // List directory
    let entries = read_dir(&sandbox, "data/nested/deep").unwrap();
    assert!(entries.contains(&"config.json".to_string()));

    // Stat directory
    let dir_info = stat(&sandbox, "data/nested").unwrap();
    assert!(dir_info.is_dir);
    assert!(!dir_info.is_file);

    // Remove file
    remove(&sandbox, "hello.txt").unwrap();
    assert!(read_file_text(&sandbox, "hello.txt").is_err());

    // Remove directory tree
    remove(&sandbox, "data").unwrap();
    assert!(stat(&sandbox, "data").is_err());

    cleanup(&sandbox);
}

#[test]
fn traversal_after_creating_files_still_blocked() {
    let sandbox = create_sandbox();

    // Create a file in the sandbox
    write_file(&sandbox, "legitimate.txt", b"safe content").unwrap();

    // Attempting traversal should still fail even after file creation
    assert!(read_file_text(&sandbox, "../../../etc/passwd").is_err());
    assert!(write_file(&sandbox, "../../evil.txt", b"pwned").is_err());
    assert!(stat(&sandbox, "/etc/passwd").is_err());
    assert!(mkdir(&sandbox, "C:\\Windows\\Temp\\evil").is_err());

    // The legitimate file should still be accessible
    let content = read_file_text(&sandbox, "legitimate.txt").unwrap();
    assert_eq!(content, "safe content");

    cleanup(&sandbox);
}

#[test]
fn safe_resolve_rejects_symlink_escape() {
    let sandbox = create_sandbox();
    let outside = create_sandbox();
    let link_path = sandbox.join("escape-link");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, &link_path).unwrap();
    }

    #[cfg(windows)]
    {
        if std::os::windows::fs::symlink_dir(&outside, &link_path).is_err() {
            // Some CI/hosts disallow symlink creation without elevated privileges.
            cleanup(&sandbox);
            cleanup(&outside);
            return;
        }
    }

    let resolved = safe_resolve(&sandbox, "escape-link/secret.txt");
    assert!(resolved.is_err());

    cleanup(&sandbox);
    cleanup(&outside);
}
