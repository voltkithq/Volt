use super::*;

#[test]
fn open_requires_database_permission() {
    let _guard = test_guard();
    configure_permissions(&[]);
    let relative_path = unique_db_path("permission-denied");
    let result = open_database(&relative_path);
    assert!(result.is_err());
    assert!(
        result
            .err()
            .is_some_and(|message| message.contains("Permission denied"))
    );
}

#[test]
fn open_allows_database_permission_without_filesystem_permission() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("db-permission");
    let _ = close_database();

    open_database(&relative_path).expect("open database with db permission");
    assert!(with_connection(|_| Ok(())).is_ok());

    close_database().expect("close database");
    cleanup_database(&relative_path);
}
