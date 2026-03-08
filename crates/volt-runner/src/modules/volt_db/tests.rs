use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::types::Value as SqlValue;
use serde_json::{Value, json};

use super::security::resolve_database_path;
use super::sql::{execute_sql, query_one_sql, query_sql, run_transaction};
use super::state::{close_database, open_database, with_connection};

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn configure_permissions(permissions: &[&str]) {
    crate::modules::configure(crate::modules::ModuleConfig {
        fs_base_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        ..Default::default()
    })
    .expect("configure module permissions");
}

fn unique_db_path(prefix: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("tests/{prefix}-{}-{nonce}.sqlite", std::process::id())
}

fn cleanup_database(relative_path: &str) {
    if let Ok(path) = resolve_database_path(relative_path) {
        let _ = fs::remove_file(path);
    }
}

#[path = "tests/crud.rs"]
mod crud;
#[path = "tests/lifecycle.rs"]
mod lifecycle;
#[path = "tests/permissions.rs"]
mod permissions;
#[path = "tests/security.rs"]
mod security;
#[path = "tests/transactions.rs"]
mod transactions;
