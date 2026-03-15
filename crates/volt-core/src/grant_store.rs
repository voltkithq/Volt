//! Process-lifetime grant store for scoped filesystem access.
//!
//! Grants are opaque tokens that authorize filesystem access to a specific
//! directory. They are created when a user selects a folder via a dialog
//! with `grantFsScope: true` and are consumed by `bindScope()` in the
//! backend runtime to create scoped file handles.
//!
//! Grants live only for the duration of the process — they are not
//! persisted, signed, or shared across processes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GrantError {
    #[error("FS_SCOPE_INVALID: grant ID not found or expired")]
    InvalidGrant,

    #[error("FS_SCOPE_INVALID: grant path does not exist or is not a directory")]
    InvalidPath,
}

/// A filesystem scope grant entry.
#[derive(Debug, Clone)]
struct GrantEntry {
    root_path: PathBuf,
}

static GRANT_COUNTER: AtomicU64 = AtomicU64::new(0);
static GRANT_STORE: Mutex<Option<HashMap<String, GrantEntry>>> = Mutex::new(None);

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, GrantEntry>) -> R,
{
    let mut guard = GRANT_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

fn generate_grant_id() -> String {
    let count = GRANT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("grant_{ts:x}_{count:x}")
}

/// Create a new grant for the given directory path.
/// Returns an opaque grant ID.
///
/// The path must exist and be a directory.
pub fn create_grant(path: PathBuf) -> Result<String, GrantError> {
    if !path.is_dir() {
        return Err(GrantError::InvalidPath);
    }

    let id = generate_grant_id();
    with_store(|store| {
        store.insert(id.clone(), GrantEntry { root_path: path });
    });
    Ok(id)
}

/// Resolve a grant ID to its root path.
/// Returns `None` if the grant does not exist.
pub fn resolve_grant(id: &str) -> Result<PathBuf, GrantError> {
    with_store(|store| {
        store
            .get(id)
            .map(|entry| entry.root_path.clone())
            .ok_or(GrantError::InvalidGrant)
    })
}

/// Revoke a grant, removing it from the store.
/// Returns true if the grant existed and was removed.
pub fn revoke_grant(id: &str) -> bool {
    with_store(|store| store.remove(id).is_some())
}

/// Clear all grants. Intended for testing and app shutdown.
pub fn clear_grants() {
    with_store(|store| store.clear());
}

/// Get the number of active grants.
pub fn grant_count() -> usize {
    with_store(|store| store.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    use crate::test_support::lock_grant_state;

    #[test]
    fn test_create_and_resolve_grant() {
        let _guard = lock_grant_state();
        let dir = env::temp_dir();
        let id = create_grant(dir.clone()).unwrap();
        assert!(id.starts_with("grant_"));

        let resolved = resolve_grant(&id).unwrap();
        assert_eq!(resolved, dir);
    }

    #[test]
    fn test_resolve_invalid_grant() {
        let _guard = lock_grant_state();
        let result = resolve_grant("nonexistent_grant_id");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FS_SCOPE_INVALID"));
    }

    #[test]
    fn test_revoke_grant() {
        let _guard = lock_grant_state();
        let dir = env::temp_dir();
        let id = create_grant(dir).unwrap();
        assert!(revoke_grant(&id));
        assert!(!revoke_grant(&id)); // second revoke returns false
        assert!(resolve_grant(&id).is_err());
    }

    #[test]
    fn test_create_grant_rejects_nonexistent_path() {
        let _guard = lock_grant_state();
        let bad_path = PathBuf::from("/definitely/does/not/exist/volt_test_grant");
        let result = create_grant(bad_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_grant_rejects_file_path() {
        let _guard = lock_grant_state();
        let base = env::temp_dir();
        let file_path = base.join("volt_test_grant_file.txt");
        std::fs::write(&file_path, b"test").unwrap();

        let result = create_grant(file_path.clone());
        assert!(result.is_err());

        std::fs::remove_file(&file_path).unwrap();
    }

    #[test]
    fn test_grant_ids_are_unique() {
        let _guard = lock_grant_state();
        let dir = env::temp_dir();
        let id1 = create_grant(dir.clone()).unwrap();
        let id2 = create_grant(dir).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_clear_grants() {
        let _guard = lock_grant_state();
        let dir = env::temp_dir();
        let _id1 = create_grant(dir.clone()).unwrap();
        let _id2 = create_grant(dir).unwrap();
        assert!(grant_count() >= 2);

        clear_grants();
        assert_eq!(grant_count(), 0);
    }
}
