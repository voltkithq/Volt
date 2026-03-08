use super::*;

#[test]
fn transaction_commit_and_rollback_behave_correctly() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("transaction");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    execute_sql("CREATE TABLE txn (id INTEGER PRIMARY KEY, value TEXT)", &[])
        .expect("create table");

    run_transaction(|| {
        execute_sql(
            "INSERT INTO txn (value) VALUES (?)",
            &[SqlValue::Text("committed".to_string())],
        )?;
        Ok(())
    })
    .expect("commit transaction");

    let first_count = query_one_sql("SELECT COUNT(*) AS total FROM txn", &[])
        .expect("query count")
        .expect("count row");
    assert_eq!(first_count["total"], Value::Number(1_i64.into()));

    let rollback_result: Result<(), String> = run_transaction(|| {
        execute_sql(
            "INSERT INTO txn (value) VALUES (?)",
            &[SqlValue::Text("rolled-back".to_string())],
        )?;
        Err("force rollback".to_string())
    });
    assert!(rollback_result.is_err());

    let second_count = query_one_sql("SELECT COUNT(*) AS total FROM txn", &[])
        .expect("query count after rollback")
        .expect("count row");
    assert_eq!(second_count["total"], Value::Number(1_i64.into()));

    close_database().expect("close database");
    cleanup_database(&relative_path);
}

#[test]
fn transaction_returns_callback_result() {
    let _guard = test_guard();
    configure_permissions(&["db"]);
    let relative_path = unique_db_path("transaction-result");
    let _ = close_database();
    open_database(&relative_path).expect("open database");

    let transaction_result = run_transaction(|| {
        execute_sql("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY)", &[])?;
        Ok(json!({ "ok": true }))
    })
    .expect("transaction result");

    assert_eq!(transaction_result, json!({ "ok": true }));
    close_database().expect("close database");
    cleanup_database(&relative_path);
}
