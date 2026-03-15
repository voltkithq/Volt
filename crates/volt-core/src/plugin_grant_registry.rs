use std::collections::HashMap;
use std::sync::Mutex;

use thiserror::Error;

use crate::grant_store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDelegation {
    pub grant_id: String,
    pub plugin_id: String,
    pub delegated_at: u64,
    pub revoked: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginGrantError {
    #[error("PLUGIN_GRANT_INVALID: grant ID does not exist")]
    InvalidGrant,
    #[error("PLUGIN_GRANT_INVALID: grant is already delegated to this plugin")]
    AlreadyDelegated,
}

static PLUGIN_GRANTS: Mutex<Option<HashMap<String, HashMap<String, GrantDelegation>>>> =
    Mutex::new(None);

fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, HashMap<String, GrantDelegation>>) -> R,
{
    let mut guard = PLUGIN_GRANTS
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn delegate_grant(plugin_id: &str, grant_id: &str) -> Result<(), PluginGrantError> {
    if grant_store::resolve_grant(grant_id).is_err() {
        return Err(PluginGrantError::InvalidGrant);
    }

    with_store(|store| {
        let delegations = store.entry(plugin_id.to_string()).or_default();
        match delegations.get_mut(grant_id) {
            Some(delegation) if !delegation.revoked => Err(PluginGrantError::AlreadyDelegated),
            Some(delegation) => {
                delegation.delegated_at = now_ms();
                delegation.revoked = false;
                Ok(())
            }
            None => {
                delegations.insert(
                    grant_id.to_string(),
                    GrantDelegation {
                        grant_id: grant_id.to_string(),
                        plugin_id: plugin_id.to_string(),
                        delegated_at: now_ms(),
                        revoked: false,
                    },
                );
                Ok(())
            }
        }
    })
}

pub fn revoke_grant(plugin_id: &str, grant_id: &str) {
    with_store(|store| {
        if let Some(delegations) = store.get_mut(plugin_id)
            && let Some(delegation) = delegations.get_mut(grant_id)
        {
            delegation.revoked = true;
        }
    });
}

pub fn revoke_all_grants(plugin_id: &str) {
    with_store(|store| {
        if let Some(delegations) = store.get_mut(plugin_id) {
            for delegation in delegations.values_mut() {
                delegation.revoked = true;
            }
        }
    });
}

pub fn is_delegated(plugin_id: &str, grant_id: &str) -> bool {
    with_store(|store| {
        store
            .get(plugin_id)
            .and_then(|delegations| delegations.get(grant_id))
            .is_some_and(|delegation| !delegation.revoked)
    })
}

pub fn list_delegated_grants(plugin_id: &str) -> Vec<String> {
    with_store(|store| {
        let mut grant_ids = store
            .get(plugin_id)
            .into_iter()
            .flat_map(|delegations| delegations.values())
            .filter(|delegation| !delegation.revoked)
            .map(|delegation| delegation.grant_id.clone())
            .collect::<Vec<_>>();
        grant_ids.sort();
        grant_ids
    })
}

pub fn clear_delegations() {
    with_store(|store| store.clear());
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::test_support::lock_grant_state;

    fn create_grant() -> String {
        let path = std::env::temp_dir().join(format!("volt-plugin-grants-{}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp dir");
        grant_store::create_grant(PathBuf::from(&path)).expect("grant")
    }

    #[test]
    fn delegate_requires_existing_grant() {
        let _guard = lock_grant_state();
        let error = delegate_grant("acme.search", "missing").expect_err("invalid grant");
        assert_eq!(error, PluginGrantError::InvalidGrant);
    }

    #[test]
    fn delegate_and_list_track_active_grants() {
        let _guard = lock_grant_state();
        let grant_id = create_grant();

        delegate_grant("acme.search", &grant_id).expect("delegate");

        assert!(is_delegated("acme.search", &grant_id));
        assert_eq!(list_delegated_grants("acme.search"), vec![grant_id]);
    }

    #[test]
    fn duplicate_delegate_is_rejected_until_revoked() {
        let _guard = lock_grant_state();
        let grant_id = create_grant();

        delegate_grant("acme.search", &grant_id).expect("delegate");
        let error = delegate_grant("acme.search", &grant_id).expect_err("duplicate");
        assert_eq!(error, PluginGrantError::AlreadyDelegated);

        revoke_grant("acme.search", &grant_id);
        delegate_grant("acme.search", &grant_id).expect("re-delegate");
        assert!(is_delegated("acme.search", &grant_id));
    }

    #[test]
    fn revoke_marks_delegation_inactive_idempotently() {
        let _guard = lock_grant_state();
        let grant_id = create_grant();
        delegate_grant("acme.search", &grant_id).expect("delegate");

        revoke_grant("acme.search", &grant_id);
        revoke_grant("acme.search", &grant_id);

        assert!(!is_delegated("acme.search", &grant_id));
        assert!(list_delegated_grants("acme.search").is_empty());
        assert!(grant_store::resolve_grant(&grant_id).is_ok());
    }

    #[test]
    fn revoke_all_marks_all_plugin_grants_inactive() {
        let _guard = lock_grant_state();
        let first = create_grant();
        let second = create_grant();
        delegate_grant("acme.search", &first).expect("delegate first");
        delegate_grant("acme.search", &second).expect("delegate second");

        revoke_all_grants("acme.search");

        assert!(!is_delegated("acme.search", &first));
        assert!(!is_delegated("acme.search", &second));
        assert!(grant_store::resolve_grant(&first).is_ok());
        assert!(grant_store::resolve_grant(&second).is_ok());
    }
}
