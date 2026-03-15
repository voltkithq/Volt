use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use volt_core::ipc::IpcRequest;

use super::*;

#[test]
fn plugin_requests_register_namespaced_command_and_ipc_entries() {
    let (manager, _factory) = manager_for_registration_tests(
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );

    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:register-command",
                json!({ "id": "search.reindex" }),
            ),
        )
        .expect("register command response");
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
        .expect("register ipc response");
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
        .expect("subscribe event response");
    assert!(response.error.is_none());

    assert_eq!(
        manager.registered_commands(),
        vec!["plugin:acme.search:search.reindex".to_string()]
    );
    assert_eq!(
        manager.registered_ipc_handlers(),
        vec!["plugin:acme.search:search.query".to_string()]
    );
    assert!(manager.has_event_subscription("acme.search", "menu:click"));
}

#[test]
fn namespaced_command_invocation_routes_to_plugin_process() {
    let requests_seen = Arc::new(Mutex::new(Vec::new()));
    let (manager, _factory) =
        manager_for_registration_tests(requests_seen.clone(), Arc::new(Mutex::new(Vec::new())));
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
        .invoke_command(
            "plugin:acme.search:search.reindex",
            json!({ "force": true }),
            Duration::from_millis(50),
        )
        .expect("command response");

    assert_eq!(response, json!({ "ok": true }));
    let seen = requests_seen.lock().expect("requests").clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "plugin:invoke-command");
    assert_eq!(
        seen[0].1,
        json!({ "id": "search.reindex", "args": { "force": true } })
    );
}

#[test]
fn frontend_ipc_routes_only_registered_plugin_handlers() {
    let requests_seen = Arc::new(Mutex::new(Vec::new()));
    let (manager, _factory) =
        manager_for_registration_tests(requests_seen.clone(), Arc::new(Mutex::new(Vec::new())));
    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:register-ipc",
                json!({ "channel": "search.query" }),
            ),
        )
        .expect("register ipc");
    assert!(response.error.is_none());

    let response = manager
        .handle_ipc_request(
            &IpcRequest {
                id: "invoke-1".to_string(),
                method: "plugin:acme.search:search.query".to_string(),
                args: json!({ "term": "volt" }),
            },
            Duration::from_millis(50),
        )
        .expect("ipc response");

    assert_eq!(response.result, Some(json!({ "result": "pong" })));
    let seen = requests_seen.lock().expect("requests").clone();
    assert_eq!(seen[0].0, "plugin:invoke-ipc");
    assert_eq!(
        seen[0].1,
        json!({ "channel": "search.query", "args": { "term": "volt" } })
    );
}
