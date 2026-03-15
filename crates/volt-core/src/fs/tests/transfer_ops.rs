use super::*;
use std::env;

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
