use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use thiserror::Error;

use crate::grant_store;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginGrantError {
    #[error("PLUGIN_GRANT_INVALID: grant ID does not exist")]
    InvalidGrant,
    #[error("PLUGIN_GRANT_INVALID: grant is already delegated to this plugin")]
    AlreadyDelegated,
}

static PLUGIN_GRANTS: Mutex<Option<HashMap<String, HashSet<String>>>> = Mutex::new(None);

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, HashSet<String>>) -> R,
{
    let mut guard = PLUGIN_GRANTS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

pub fn delegate_grant(plugin_id: &str, grant_id: &str) -> Result<(), PluginGrantError> {
    if grant_store::resolve_grant(grant_id).is_err() {
        return Err(PluginGrantError::InvalidGrant);
    }

    with_store(|store| {
        let grants = store.entry(plugin_id.to_string()).or_default();
        if !grants.insert(grant_id.to_string()) {
            return Err(PluginGrantError::AlreadyDelegated);
        }
        Ok(())
    })
}

pub fn is_delegated(plugin_id: &str, grant_id: &str) -> bool {
    with_store(|store| {
        store
            .get(plugin_id)
            .is_some_and(|grants| grants.contains(grant_id))
    })
}

pub fn delegated_grants(plugin_id: &str) -> Vec<String> {
    with_store(|store| {
        let mut grants = store
            .get(plugin_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        grants.sort();
        grants
    })
}

pub fn revoke_grant(plugin_id: &str, grant_id: &str) {
    with_store(|store| {
        if let Some(grants) = store.get_mut(plugin_id) {
            grants.remove(grant_id);
            if grants.is_empty() {
                store.remove(plugin_id);
            }
        }
    });
}

pub fn revoke_all(plugin_id: &str) {
    with_store(|store| {
        store.remove(plugin_id);
    });
}

pub fn clear_delegations() {
    with_store(|store| store.clear());
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn delegate_grant_requires_existing_grant() {
        clear_delegations();
        grant_store::clear_grants();

        let error = delegate_grant("acme.search", "missing").expect_err("invalid grant");
        assert_eq!(error, PluginGrantError::InvalidGrant);
    }

    #[test]
    fn delegate_grant_tracks_plugin_ownership() {
        clear_delegations();
        grant_store::clear_grants();

        let grant_id = grant_store::create_grant(std::env::temp_dir()).expect("grant");
        delegate_grant("acme.search", &grant_id).expect("delegate");

        assert!(is_delegated("acme.search", &grant_id));
        assert_eq!(delegated_grants("acme.search"), vec![grant_id.clone()]);
        assert!(!is_delegated("beta.index", &grant_id));
    }

    #[test]
    fn duplicate_delegation_is_rejected() {
        clear_delegations();
        grant_store::clear_grants();

        let temp = std::env::temp_dir().join("volt-plugin-grants");
        std::fs::create_dir_all(&temp).expect("temp");
        let grant_id = grant_store::create_grant(PathBuf::from(&temp)).expect("grant");

        delegate_grant("acme.search", &grant_id).expect("delegate");
        let error = delegate_grant("acme.search", &grant_id).expect_err("duplicate");
        assert_eq!(error, PluginGrantError::AlreadyDelegated);

        revoke_grant("acme.search", &grant_id);
        assert!(!is_delegated("acme.search", &grant_id));

        let _ = std::fs::remove_dir_all(temp);
    }
}
