use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use volt_core::{grant_store, ipc::IpcRequest};

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory};
use super::shared::{lock_grant_state, manager_with_factory, register_ipc_handler};
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

fn grant_manager(factory: Arc<FakeProcessFactory>) -> PluginManager {
    let root = TempDir::new("plugin-grant-delegation");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory,
    )
}

fn bind_grant(manager: &PluginManager, grant_id: &str) -> Result<serde_json::Value, String> {
    let response = manager
        .handle_plugin_message(
            "acme.search",
            WireMessage {
                message_type: WireMessageType::Request,
                id: "bind-grant".to_string(),
                method: "plugin:bind-grant".to_string(),
                payload: Some(json!({ "grantId": grant_id })),
                error: None,
            },
        )
        .expect("bind response");
    match response.error {
        Some(error) => Err(error.message),
        None => Ok(response.payload.expect("bind payload")),
    }
}

#[test]
fn delegate_grant_allows_plugin_bind_and_list() {
    let _guard = lock_grant_state();
    let selected = TempDir::new("delegated-root");
    let grant_id = grant_store::create_grant(selected.path().to_path_buf()).expect("grant");
    let manager = grant_manager(Arc::new(FakeProcessFactory::new(HashMap::new())));

    manager
        .delegate_grant("acme.search", &grant_id)
        .expect("delegate");

    let payload = bind_grant(&manager, &grant_id).expect("bind grant");
    assert_eq!(payload["grantId"].as_str(), Some(grant_id.as_str()));
    assert_eq!(
        manager
            .list_delegated_grants("acme.search")
            .expect("list grants"),
        vec![grant_id]
    );
}

#[test]
fn revoke_grant_notifies_running_plugin_and_blocks_future_bind() {
    let _guard = lock_grant_state();
    let selected = TempDir::new("revoked-root");
    let grant_id = grant_store::create_grant(selected.path().to_path_buf()).expect("grant");
    let plan = FakePlan::default();
    let sent_events = plan.sent_events.clone();
    let factory = Arc::new(FakeProcessFactory::new(HashMap::from([(
        "acme.search".to_string(),
        plan,
    )])));
    let manager = grant_manager(factory);

    manager
        .delegate_grant("acme.search", &grant_id)
        .expect("delegate");
    register_ipc_handler(&manager, "acme.search", "search.query");
    let _ = manager.handle_ipc_request(
        &IpcRequest {
            id: "spawn".to_string(),
            method: "plugin:acme.search:search.query".to_string(),
            args: json!(null),
        },
        Duration::from_millis(50),
    );

    manager
        .revoke_grant("acme.search", &grant_id)
        .expect("revoke");

    let events = sent_events.lock().expect("sent events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "plugin:grant-revoked");
    assert_eq!(events[0].1["grantId"].as_str(), Some(grant_id.as_str()));
    drop(events);

    let error = bind_grant(&manager, &grant_id).expect_err("bind denied");
    assert!(error.contains("not delegated"));
}

#[test]
fn deactivation_revokes_all_grants_without_deleting_the_underlying_grant() {
    let _guard = lock_grant_state();
    let selected = TempDir::new("shutdown-root");
    let grant_id = grant_store::create_grant(selected.path().to_path_buf()).expect("grant");
    let manager = grant_manager(Arc::new(FakeProcessFactory::new(HashMap::new())));

    manager
        .delegate_grant("acme.search", &grant_id)
        .expect("delegate");
    manager.shutdown_all();

    assert!(
        manager
            .list_delegated_grants("acme.search")
            .expect("list after shutdown")
            .is_empty()
    );
    assert!(bind_grant(&manager, &grant_id).is_err());
    assert!(grant_store::resolve_grant(&grant_id).is_ok());
}
