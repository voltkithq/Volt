use serde_json::Value;

use crate::config::{HostIpcSettings, PluginConfig};
use crate::ipc::IpcMessage;
use crate::runtime_state::{configure_mock, send_request, take_outbound};

fn test_config() -> PluginConfig {
    PluginConfig {
        plugin_id: "acme.search".into(),
        backend_entry: "./dist/plugin.js".into(),
        manifest: serde_json::json!({ "id": "acme.search" }),
        capabilities: vec!["fs".into()],
        data_root: ".".into(),
        delegated_grants: vec![],
        host_ipc_settings: None,
    }
}

fn test_config_with_queue_depth(max_queue_depth: u32) -> PluginConfig {
    let mut config = test_config();
    config.host_ipc_settings = Some(HostIpcSettings {
        max_queue_depth,
        ..HostIpcSettings::default()
    });
    config
}

#[test]
fn send_request_writes_request_and_reads_response() {
    configure_mock(
        &test_config(),
        vec![IpcMessage::response(
            "plugin-request-1",
            "plugin:register-command",
            Some(serde_json::json!(true)),
        )],
    );

    let response = send_request(
        "plugin:register-command",
        serde_json::json!({ "id": "search.reindex" }),
    )
    .expect("response");

    assert_eq!(response, serde_json::json!(true));
    let outbound = take_outbound();
    assert_eq!(outbound.len(), 1);
    assert_eq!(outbound[0].method, "plugin:register-command");
}

#[test]
fn send_request_acks_heartbeat_while_waiting() {
    configure_mock(
        &test_config(),
        vec![
            IpcMessage::signal("hb-1", "heartbeat"),
            IpcMessage::response(
                "plugin-request-1",
                "plugin:fs:exists",
                Some(Value::Bool(true)),
            ),
        ],
    );

    let exists =
        send_request("plugin:fs:exists", serde_json::json!({ "path": "cache" })).expect("exists");
    assert_eq!(exists, Value::Bool(true));

    let outbound = take_outbound();
    assert_eq!(outbound.len(), 2);
    assert_eq!(outbound[1].method, "heartbeat-ack");
}

#[test]
fn send_request_rejects_when_deferred_queue_exceeds_limit() {
    configure_mock(
        &test_config_with_queue_depth(1),
        vec![
            IpcMessage::signal("event-1", "plugin:event"),
            IpcMessage::signal("event-2", "plugin:event"),
        ],
    );

    let error = send_request("plugin:fs:exists", serde_json::json!({ "path": "cache" }))
        .expect_err("error");

    assert!(error.contains("deferred message queue exceeded 1 messages"));
}
