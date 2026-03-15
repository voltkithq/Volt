use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use volt_core::ipc::IpcRequest;

use super::*;

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
fn plugin_emitted_events_route_to_namespaced_subscribers() {
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

    let events = sent_events.lock().expect("events").clone();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "plugin:event");
    assert_eq!(
        events[0].1,
        json!({ "event": "search:done", "data": { "count": 3 } })
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
