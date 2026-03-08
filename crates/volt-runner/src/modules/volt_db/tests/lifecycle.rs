use super::*;

#[test]
fn open_and_close_lifecycle() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("lifecycle");
    let _ = close_database();

    open_database(&relative_path).expect("open database");
    assert!(with_connection(|_| Ok(())).is_ok());

    close_database().expect("close database");
    assert!(with_connection(|_| Ok(())).is_err());

    cleanup_database(&relative_path);
}

#[test]
fn query_one_returns_none_when_no_rows_match() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("query-one-none");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql("CREATE TABLE t (id INTEGER PRIMARY KEY)", &[]).expect("create table");
    let row = query_one_sql("SELECT id FROM t WHERE id = ?", &[SqlValue::Integer(99)])
        .expect("query no rows");
    assert!(row.is_none());

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn database_persists_across_open_close_cycles() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("persistence");
    let _ = close_database();

    open_database(&relative_path).expect("open database");
    execute_sql(
        "CREATE TABLE persisted (id INTEGER PRIMARY KEY, value TEXT)",
        &[],
    )
    .expect("create table");
    execute_sql(
        "INSERT INTO persisted (value) VALUES (?)",
        &[SqlValue::Text("saved".to_string())],
    )
    .expect("insert row");
    close_database().expect("close database");

    open_database(&relative_path).expect("reopen database");
    let row = query_one_sql("SELECT COUNT(*) AS total FROM persisted", &[])
        .expect("query persisted rows")
        .expect("count row");
    assert_eq!(row["total"], Value::Number(1_i64.into()));

    close_database().expect("final close");
    cleanup_database(&relative_path);
}
