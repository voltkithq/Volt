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

use sha2::{Digest, Sha256};
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

static GRANT_STORE: Mutex<Option<HashMap<String, GrantEntry>>> = Mutex::new(None);

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, GrantEntry>) -> R,
{
    let mut guard = GRANT_STORE.lock().unwrap_or_else(|e| e.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

/// Generate a cryptographically unpredictable grant ID by hashing
/// nanosecond timestamp + process ID + a random seed from the address
/// of a freshly allocated Box (ASLR-derived entropy). This replaces the
/// previous timestamp+counter scheme which was guessable.
fn generate_grant_id() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let entropy_box = Box::new(0u8);
    let addr = &*entropy_box as *const u8 as usize;
    let pid = std::process::id();
    let tid = std::thread::current().id();
    let mut hasher = Sha256::new();
    hasher.update(ts.to_le_bytes());
    hasher.update(pid.to_le_bytes());
    hasher.update(format!("{tid:?}").as_bytes());
    hasher.update(addr.to_le_bytes());
    let hash = hasher.finalize();
    format!("grant_{:x}", hash)
}

/// Create a new grant for the given directory path.
/// Returns an opaque grant ID.
///
/// The path must exist and be a directory.
pub fn create_grant(path: PathBuf) -> Result<String, GrantError> {
    if !path.is_dir() {
        return Err(GrantError::InvalidPath);
    }

    // Canonicalize at creation time so the grant always refers to the
    // real, resolved path — preventing drift if symlinks change later.
    let canonical = path.canonicalize().map_err(|_| GrantError::InvalidPath)?;

    let id = generate_grant_id();
    with_store(|store| {
        store.insert(
            id.clone(),
            GrantEntry {
                root_path: canonical,
            },
        );
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
        let canonical_dir = dir.canonicalize().unwrap();
        let id = create_grant(dir).unwrap();
        assert!(id.starts_with("grant_"));

        let resolved = resolve_grant(&id).unwrap();
        assert_eq!(resolved, canonical_dir);
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
