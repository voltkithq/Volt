use base64::Engine;
use boa_engine::{Context, JsValue};
use rusqlite::params_from_iter;
use rusqlite::types::{Value as SqlValue, ValueRef};
use serde_json::{Map, Value, json};

use crate::modules::value_to_json;

use super::state::with_connection;

const MAX_SQL_LENGTH: usize = 1_048_576;
const MAX_QUERY_ROWS: usize = 10_000;

fn normalize_sql(sql: &str) -> Result<String, String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("SQL statement must not be empty".to_string());
    }
    if trimmed.len() > MAX_SQL_LENGTH {
        return Err(format!(
            "SQL statement exceeds maximum length ({MAX_SQL_LENGTH} bytes)"
        ));
    }
    Ok(trimmed.to_string())
}

fn json_value_to_sql_value(value: &Value) -> Result<SqlValue, String> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::Bool(flag) => Ok(SqlValue::Integer(if *flag { 1 } else { 0 })),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(SqlValue::Integer(integer))
            } else if let Some(real) = number.as_f64() {
                Ok(SqlValue::Real(real))
            } else {
                Err(format!("unsupported JSON number value: {number}"))
            }
        }
        Value::String(text) => Ok(SqlValue::Text(text.clone())),
        Value::Object(object) => {
            if let Some(encoded_blob) = object.get("$blob").and_then(Value::as_str) {
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(encoded_blob)
                    .map_err(|error| format!("invalid base64 blob parameter: {error}"))?;
                Ok(SqlValue::Blob(decoded))
            } else {
                serde_json::to_string(value)
                    .map(SqlValue::Text)
                    .map_err(|error| format!("failed to serialize JSON object parameter: {error}"))
            }
        }
        Value::Array(_) => serde_json::to_string(value)
            .map(SqlValue::Text)
            .map_err(|error| format!("failed to serialize JSON array parameter: {error}")),
    }
}

pub(super) fn parse_sql_params(
    params: Option<JsValue>,
    context: &mut Context,
) -> Result<Vec<SqlValue>, String> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };

    let json_params = value_to_json(params, context)?;
    let values = json_params
        .as_array()
        .ok_or_else(|| "SQL parameters must be an array".to_string())?;

    values.iter().map(json_value_to_sql_value).collect()
}

pub(super) fn execute_sql(sql: &str, params: &[SqlValue]) -> Result<u64, String> {
    let normalized_sql = normalize_sql(sql)?;
    with_connection(|connection| {
        let mut statement = connection
            .prepare(&normalized_sql)
            .map_err(|error| format!("failed to prepare SQL statement: {error}"))?;
        let rows_affected = statement
            .execute(params_from_iter(params.iter()))
            .map_err(|error| format!("failed to execute SQL statement: {error}"))?;
        Ok(rows_affected as u64)
    })
}

pub(super) fn query_sql(sql: &str, params: &[SqlValue]) -> Result<Vec<Value>, String> {
    let normalized_sql = normalize_sql(sql)?;
    with_connection(|connection| {
        let mut statement = connection
            .prepare(&normalized_sql)
            .map_err(|error| format!("failed to prepare SQL query: {error}"))?;
        let column_count = statement.column_count();
        let column_names = collect_column_names(&statement)?;

        let mut rows = statement
            .query(params_from_iter(params.iter()))
            .map_err(|error| format!("failed to execute SQL query: {error}"))?;
        let mut output_rows = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|error| format!("failed to read SQL row: {error}"))?
        {
            if output_rows.len() >= MAX_QUERY_ROWS {
                return Err(format!(
                    "query result exceeds maximum row count ({MAX_QUERY_ROWS})"
                ));
            }
            let mut object = Map::with_capacity(column_count);
            for (index, column_name) in column_names.iter().enumerate() {
                let value_ref = row
                    .get_ref(index)
                    .map_err(|error| format!("failed to read SQL column value: {error}"))?;
                object.insert(column_name.clone(), sqlite_value_ref_to_json(value_ref));
            }
            output_rows.push(Value::Object(object));
        }

        Ok(output_rows)
    })
}

pub(super) fn query_one_sql(sql: &str, params: &[SqlValue]) -> Result<Option<Value>, String> {
    let normalized_sql = normalize_sql(sql)?;
    with_connection(|connection| {
        let mut statement = connection
            .prepare(&normalized_sql)
            .map_err(|error| format!("failed to prepare SQL query: {error}"))?;
        let column_count = statement.column_count();
        let column_names = collect_column_names(&statement)?;

        let mut rows = statement
            .query(params_from_iter(params.iter()))
            .map_err(|error| format!("failed to execute SQL query: {error}"))?;
        let Some(row) = rows
            .next()
            .map_err(|error| format!("failed to read SQL row: {error}"))?
        else {
            return Ok(None);
        };

        let mut object = Map::with_capacity(column_count);
        for (index, column_name) in column_names.iter().enumerate() {
            let value_ref = row
                .get_ref(index)
                .map_err(|error| format!("failed to read SQL column value: {error}"))?;
            object.insert(column_name.clone(), sqlite_value_ref_to_json(value_ref));
        }
        Ok(Some(Value::Object(object)))
    })
}

pub(super) fn run_transaction<T>(
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    begin_transaction()?;
    match operation() {
        Ok(result) => {
            if let Err(error) = commit_transaction() {
                let _ = rollback_transaction();
                return Err(error);
            }
            Ok(result)
        }
        Err(error) => {
            let _ = rollback_transaction();
            Err(error)
        }
    }
}

fn sqlite_value_ref_to_json(value_ref: ValueRef<'_>) -> Value {
    match value_ref {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(integer) => json!(integer),
        ValueRef::Real(real) => json!(real),
        ValueRef::Text(text) => Value::String(String::from_utf8_lossy(text).to_string()),
        ValueRef::Blob(blob) => json!({
            "$blob": base64::engine::general_purpose::STANDARD.encode(blob)
        }),
    }
}

fn collect_column_names(statement: &rusqlite::Statement<'_>) -> Result<Vec<String>, String> {
    let column_count = statement.column_count();
    (0..column_count)
        .map(|index| {
            statement
                .column_name(index)
                .map(|name| name.to_string())
                .map_err(|error| {
                    format!("failed to read SQL column name at index {index}: {error}")
                })
        })
        .collect()
}

fn begin_transaction() -> Result<(), String> {
    with_connection(|connection| {
        connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|error| format!("failed to begin transaction: {error}"))
    })
}

fn commit_transaction() -> Result<(), String> {
    with_connection(|connection| {
        connection
            .execute_batch("COMMIT")
            .map_err(|error| format!("failed to commit transaction: {error}"))
    })
}

fn rollback_transaction() -> Result<(), String> {
    with_connection(|connection| {
        connection
            .execute_batch("ROLLBACK")
            .map_err(|error| format!("failed to rollback transaction: {error}"))
    })
}
