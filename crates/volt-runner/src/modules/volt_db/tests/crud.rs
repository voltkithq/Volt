use super::*;

#[test]
fn create_insert_select_update_delete_roundtrip() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("crud");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql(
        "CREATE TABLE IF NOT EXISTS todos (id TEXT PRIMARY KEY, text TEXT NOT NULL, done INTEGER NOT NULL)",
        &[],
    )
    .expect("create table");
    execute_sql(
        "INSERT INTO todos (id, text, done) VALUES (?, ?, ?)",
        &[
            SqlValue::Text("1".to_string()),
            SqlValue::Text("ship feature".to_string()),
            SqlValue::Integer(0),
        ],
    )
    .expect("insert todo");

    let first = query_one_sql(
        "SELECT text, done FROM todos WHERE id = ?",
        &[SqlValue::Text("1".to_string())],
    )
    .expect("query one")
    .expect("row exists");
    assert_eq!(first["text"], Value::String("ship feature".to_string()));
    assert_eq!(first["done"], Value::Number(0_i64.into()));

    execute_sql(
        "UPDATE todos SET done = ? WHERE id = ?",
        &[SqlValue::Integer(1), SqlValue::Text("1".to_string())],
    )
    .expect("update todo");
    let updated_done = query_one_sql(
        "SELECT done FROM todos WHERE id = ?",
        &[SqlValue::Text("1".to_string())],
    )
    .expect("query updated todo")
    .expect("row exists");
    assert_eq!(updated_done["done"], Value::Number(1_i64.into()));

    execute_sql(
        "DELETE FROM todos WHERE id = ?",
        &[SqlValue::Text("1".to_string())],
    )
    .expect("delete todo");
    assert!(
        query_one_sql(
            "SELECT id FROM todos WHERE id = ?",
            &[SqlValue::Text("1".to_string())]
        )
        .expect("query after delete")
        .is_none()
    );

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn parameterized_queries_block_sql_injection() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("injection");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql(
        "CREATE TABLE notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL)",
        &[],
    )
    .expect("create table");
    let malicious = "x'); DROP TABLE notes; --";
    execute_sql(
        "INSERT INTO notes (body) VALUES (?)",
        &[SqlValue::Text(malicious.to_string())],
    )
    .expect("insert note");

    let count = query_one_sql("SELECT COUNT(*) AS total FROM notes", &[])
        .expect("count rows")
        .expect("count row");
    assert_eq!(count["total"], Value::Number(1_i64.into()));

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn query_maps_sqlite_types_to_json_values() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("types");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql(
        "CREATE TABLE typed (i INTEGER, r REAL, t TEXT, b BLOB, n NULL)",
        &[],
    )
    .expect("create table");
    execute_sql(
        "INSERT INTO typed (i, r, t, b, n) VALUES (?, ?, ?, ?, ?)",
        &[
            SqlValue::Integer(7),
            SqlValue::Real(3.5),
            SqlValue::Text("hello".to_string()),
            SqlValue::Blob(vec![1, 2, 3, 4]),
            SqlValue::Null,
        ],
    )
    .expect("insert typed row");

    let row = query_one_sql("SELECT i, r, t, b, n FROM typed", &[])
        .expect("query typed row")
        .expect("row exists");
    assert_eq!(row["i"], Value::Number(7_i64.into()));
    assert_eq!(row["r"], json!(3.5));
    assert_eq!(row["t"], Value::String("hello".to_string()));
    assert_eq!(row["b"], json!({ "$blob": "AQIDBA==" }));
    assert_eq!(row["n"], Value::Null);

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn handles_repeated_read_write_operations_on_single_thread() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("single-thread");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql(
        "CREATE TABLE ops (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)",
        &[],
    )
    .expect("create table");

    for index in 0..64_i64 {
        execute_sql(
            "INSERT INTO ops (value) VALUES (?)",
            &[SqlValue::Integer(index)],
        )
        .expect("insert value");
        let count = query_one_sql("SELECT COUNT(*) AS total FROM ops", &[])
            .expect("count rows")
            .expect("count row");
        assert_eq!(count["total"], Value::Number((index + 1).into()));
    }

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn returns_errors_for_invalid_sql_constraint_and_missing_connection() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("errors");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    let invalid_sql_error = execute_sql("THIS IS NOT SQL", &[]);
    assert!(invalid_sql_error.is_err());

    execute_sql("CREATE TABLE uniq (value TEXT UNIQUE NOT NULL)", &[]).expect("create table");
    execute_sql(
        "INSERT INTO uniq (value) VALUES (?)",
        &[SqlValue::Text("v".to_string())],
    )
    .expect("first insert");
    let duplicate_error = execute_sql(
        "INSERT INTO uniq (value) VALUES (?)",
        &[SqlValue::Text("v".to_string())],
    );
    assert!(duplicate_error.is_err());

    close_database().expect("close database");
    let missing_connection_error = query_sql("SELECT 1", &[]);
    assert!(missing_connection_error.is_err());

    cleanup_database(&relative_path);
}
