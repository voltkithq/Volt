use super::*;

#[test]
fn rejects_traversal_and_absolute_database_paths() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let _ = close_database();

    assert!(open_database("../evil.db").is_err());
    assert!(open_database("/absolute.db").is_err());
    assert!(open_database("C:\\Windows\\system32\\evil.db").is_err());
}

#[test]
fn wal_mode_is_enabled_after_open() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("wal");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    let row = query_one_sql("PRAGMA journal_mode", &[])
        .expect("query pragma")
        .expect("pragma row");
    let mode = row
        .get("journal_mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert_eq!(mode, "wal");

    close_database().expect("close database");
    cleanup_database(&relative_path);
}
