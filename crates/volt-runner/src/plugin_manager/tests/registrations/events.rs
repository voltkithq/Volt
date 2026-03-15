use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use volt_core::ipc::IpcRequest;

use super::super::fs_support::{TempDir, write_manifest};
use super::super::process_support::{FakePlan, FakeProcessFactory};
use super::super::shared::manager_with_factory;
use super::*;
use crate::runner::config::RunnerPluginConfig;

fn manager_for_cross_plugin_events(
    search_events: Arc<Mutex<Vec<(String, Value)>>>,
    beta_events: Arc<Mutex<Vec<(String, Value)>>>,
) -> PluginManager {
    let root = TempDir::new("cross-plugin-events");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    write_manifest(
        &root.join("plugins/beta.index/volt-plugin.json"),
        "beta.index",
        &["fs"],
    );

    let factory = Arc::new(FakeProcessFactory::new(HashMap::from([
        (
            "acme.search".to_string(),
            FakePlan {
                sent_events: search_events,
                ..FakePlan::default()
            },
        ),
        (
            "beta.index".to_string(),
            FakePlan {
                sent_events: beta_events,
                ..FakePlan::default()
            },
        ),
    ])));

    manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string(), "beta.index".to_string()],
            grants: BTreeMap::from([
                ("acme.search".to_string(), vec!["fs".to_string()]),
                ("beta.index".to_string(), vec!["fs".to_string()]),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory,
    )
}

#[test]
fn host_events_route_to_subscribed_plugins() {
    let requests_seen = Arc::new(Mutex::new(Vec::new()));
    let sent_events = Arc::new(Mutex::new(Vec::new()));
    let (manager, _factory) =
        manager_for_registration_tests(requests_seen.clone(), sent_events.clone());
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:subscribe-event",
                json!({ "event": "menu:click" }),
            ),
        )
        .expect("subscribe");
    assert!(response.error.is_none());
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-2",
                "plugin:register-command",
                json!({ "id": "search.reindex" }),
            ),
        )
        .expect("register command");
    assert!(response.error.is_none());
    let _ = manager.invoke_command(
        "plugin:acme.search:search.reindex",
        json!({ "force": false }),
        Duration::from_millis(50),
    );

    manager.dispatch_host_event("menu:click", json!({ "menuId": "file.open" }));

    let events = sent_events.lock().expect("events").clone();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "plugin:event");
    assert_eq!(
        events[0].1,
        json!({ "event": "menu:click", "data": { "menuId": "file.open" } })
    );
}

#[test]
fn plugin_emitted_events_do_not_loop_back_to_the_emitter() {
    let sent_events = Arc::new(Mutex::new(Vec::new()));
    let (manager, _factory) =
        manager_for_registration_tests(Arc::new(Mutex::new(Vec::new())), sent_events.clone());
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:subscribe-event",
                json!({ "event": "search:done" }),
            ),
        )
        .expect("subscribe");
    assert!(response.error.is_none());

    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-2",
                "plugin:emit-event",
                json!({ "event": "search:done", "data": { "count": 3 } }),
            ),
        )
        .expect("emit response");
    assert!(response.error.is_none());

    assert!(sent_events.lock().expect("events").is_empty());
}

#[test]
fn plugin_emitted_events_route_to_other_plugins_only() {
    let search_events = Arc::new(Mutex::new(Vec::new()));
    let beta_events = Arc::new(Mutex::new(Vec::new()));
    let manager = manager_for_cross_plugin_events(search_events.clone(), beta_events.clone());

    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:subscribe-event",
                json!({ "event": "search:done" }),
            ),
        )
        .expect("search subscribe");
    assert!(response.error.is_none());
    let response = manager
        .handle_plugin_message(
            "beta.index",
            request_message(
                "req-2",
                "plugin:subscribe-event",
                json!({ "event": "search:done" }),
            ),
        )
        .expect("beta subscribe");
    assert!(response.error.is_none());

    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-3",
                "plugin:emit-event",
                json!({ "event": "search:done", "data": { "count": 7 } }),
            ),
        )
        .expect("emit response");
    assert!(response.error.is_none());

    assert!(search_events.lock().expect("search events").is_empty());
    let beta_events = beta_events.lock().expect("beta events").clone();
    assert_eq!(beta_events.len(), 1);
    assert_eq!(beta_events[0].0, "plugin:event");
    assert_eq!(
        beta_events[0].1,
        json!({ "event": "search:done", "data": { "count": 7 } })
    );
}

#[test]
fn deactivation_cleans_registered_command_event_and_ipc_entries() {
    let requests_seen = Arc::new(Mutex::new(Vec::new()));
    let sent_events = Arc::new(Mutex::new(Vec::new()));
    let (manager, _factory) = manager_for_registration_tests(requests_seen, sent_events);
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:register-command",
                json!({ "id": "search.reindex" }),
            ),
        )
        .expect("register command");
    assert!(response.error.is_none());
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-2",
                "plugin:register-ipc",
                json!({ "channel": "search.query" }),
            ),
        )
        .expect("register ipc");
    assert!(response.error.is_none());
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-3",
                "plugin:subscribe-event",
                json!({ "event": "menu:click" }),
            ),
        )
        .expect("subscribe");
    assert!(response.error.is_none());

    let _ = manager.handle_ipc_request(
        &IpcRequest {
            id: "invoke-1".to_string(),
            method: "plugin:acme.search:search.query".to_string(),
            args: Value::Null,
        },
        Duration::from_millis(50),
    );
    manager.deactivate_plugin("acme.search");

    assert!(manager.registered_commands().is_empty());
    assert!(manager.registered_ipc_handlers().is_empty());
    assert!(!manager.has_event_subscription("acme.search", "menu:click"));
}
