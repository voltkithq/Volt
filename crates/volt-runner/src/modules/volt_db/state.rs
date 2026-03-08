use std::cell::RefCell;
use std::time::Duration;

use rusqlite::Connection;

use super::security::{ensure_database_permission, resolve_database_path};

const DEFAULT_BUSY_TIMEOUT_MS: u64 = 5_000;

#[derive(Default)]
struct DatabaseState {
    connection: Option<Connection>,
}

thread_local! {
    static DB_STATE: RefCell<DatabaseState> = RefCell::new(DatabaseState::default());
}

pub(super) fn with_connection<T>(
    operation: impl FnOnce(&mut Connection) -> Result<T, String>,
) -> Result<T, String> {
    DB_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let connection = state
            .connection
            .as_mut()
            .ok_or_else(|| "database is not open. Call db.open(path) first.".to_string())?;
        operation(connection)
    })
}

pub(super) fn open_database(path: &str) -> Result<(), String> {
    ensure_database_permission()?;
    close_database_internal()?;
    let resolved_path = resolve_database_path(path)?;

    let connection = Connection::open(&resolved_path).map_err(|error| {
        format!(
            "failed to open SQLite database at '{}': {error}",
            resolved_path.display()
        )
    })?;

    connection
        .busy_timeout(Duration::from_millis(DEFAULT_BUSY_TIMEOUT_MS))
        .map_err(|error| format!("failed to configure SQLite busy timeout: {error}"))?;
    connection
        .execute_batch(
            "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        ",
        )
        .map_err(|error| format!("failed to configure SQLite pragmas: {error}"))?;

    DB_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.connection = Some(connection);
    });

    Ok(())
}

pub(super) fn close_database() -> Result<(), String> {
    ensure_database_permission()?;
    close_database_internal()
}

fn close_database_internal() -> Result<(), String> {
    DB_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let Some(connection) = state.connection.take() else {
            return Ok(());
        };

        match connection.close() {
            Ok(()) => Ok(()),
            Err((connection, error)) => {
                state.connection = Some(connection);
                Err(format!("failed to close SQLite database: {error}"))
            }
        }
    })
}
