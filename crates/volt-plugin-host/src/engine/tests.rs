use crate::config::PluginConfig;
use crate::engine::PluginEngine;
use crate::ipc::{IpcMessage, MessageType};
use crate::runtime_state::{configure_mock, take_outbound};

fn build_config(script_name: &str, source: &str) -> PluginConfig {
    let temp_dir = std::env::temp_dir().join(format!(
        "volt-plugin-host-{script_name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).expect("temp dir");
    let script_path = temp_dir.join(format!("{script_name}.mjs"));
    std::fs::write(&script_path, source).expect("plugin script");

    PluginConfig {
        plugin_id: "acme.search".into(),
        backend_entry: script_path.display().to_string(),
        manifest: serde_json::json!({ "id": "acme.search", "name": "Acme Search" }),
        capabilities: vec!["fs".into()],
        data_root: temp_dir.display().to_string(),
        delegated_grants: vec![],
        host_ipc_settings: None,
    }
}

fn build_config_with_grants(
    script_name: &str,
    source: &str,
    delegated_grants: Vec<crate::config::DelegatedGrant>,
) -> PluginConfig {
    let mut config = build_config(script_name, source);
    config.delegated_grants = delegated_grants;
    config
}

#[test]
fn activate_registers_runtime_handlers_and_invokes_command() {
    let config = build_config(
        "activate",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    context.log.info('activated');
                    context.commands.register('search.reindex', async (args) => ({ ok: args.ok }));
                    context.ipc.handle('search.query', async (args) => ({ echoed: args }));
                }
            });
        "#,
    );
    configure_mock(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:register-command",
                Some(serde_json::json!(true)),
            ),
            IpcMessage::response(
                "plugin-request-2",
                "plugin:register-ipc",
                Some(serde_json::json!(true)),
            ),
        ],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate");
    engine
        .dispatch_message(IpcMessage {
            msg_type: MessageType::Request,
            id: "invoke-1".into(),
            method: "plugin:invoke-command".into(),
            payload: Some(serde_json::json!({
                "id": "search.reindex",
                "args": { "ok": true }
            })),
            error: None,
        })
        .expect("command");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:log");
    assert_eq!(outbound[2].method, "plugin:register-command");
    assert_eq!(outbound[3].method, "plugin:register-ipc");
    assert_eq!(outbound[4].method, "activate");
    assert_eq!(outbound[5].method, "plugin:invoke-command");
}

#[test]
fn activate_uses_storage_request_access_and_grant_fs_bridge() {
    let config = build_config(
        "storage-and-grants",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    await context.storage.set('token', 'abc');
                    await context.storage.get('token');
                    const access = await context.grants.requestAccess({
                        title: 'Select search directory',
                        directory: true,
                    });
                    const scoped = context.grants.bindFsScope(access.grantId);
                    await scoped.exists('child.txt');
                }
            });
        "#,
    );
    configure_mock(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:storage:set",
                Some(serde_json::Value::Null),
            ),
            IpcMessage::response(
                "plugin-request-2",
                "plugin:storage:get",
                Some(serde_json::json!("abc")),
            ),
            IpcMessage::response(
                "plugin-request-3",
                "plugin:request-access",
                Some(serde_json::json!({
                    "grantId": "grant-1",
                    "path": "C:\\data\\search"
                })),
            ),
            IpcMessage::response(
                "plugin-request-4",
                "plugin:grant-fs:exists",
                Some(serde_json::json!(true)),
            ),
        ],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:storage:set");
    assert_eq!(outbound[2].method, "plugin:storage:get");
    assert_eq!(outbound[3].method, "plugin:request-access");
    assert_eq!(outbound[4].method, "plugin:grant-fs:exists");
    assert_eq!(outbound[5].method, "activate");
}

#[test]
fn delegated_grants_can_bind_without_requesting_access_again() {
    let config = build_config_with_grants(
        "predelegated-grants",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    const scoped = context.grants.bindFsScope('grant-1');
                    await scoped.readFile('child.txt');
                }
            });
        "#,
        vec![crate::config::DelegatedGrant {
            grant_id: "grant-1".into(),
            path: "C:\\data\\search".into(),
        }],
    );
    configure_mock(
        &config,
        vec![IpcMessage::response(
            "plugin-request-1",
            "plugin:grant-fs:read-file",
            Some(serde_json::json!("ok")),
        )],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:grant-fs:read-file");
    assert_eq!(outbound[2].method, "activate");
}
