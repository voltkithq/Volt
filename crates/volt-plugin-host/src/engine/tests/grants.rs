use crate::config::DelegatedGrant;
use crate::engine::PluginEngine;
use crate::ipc::{IpcMessage, MessageType};
use crate::runtime_state::take_outbound;

use super::{build_config, build_config_with_grants, configure_engine};

#[test]
fn activate_uses_storage_access_bind_and_grant_fs_bridge() {
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
                    const scoped = await context.grants.bindFsScope(access.grantId);
                    await scoped.exists('child.txt');
                }
            });
        "#,
    );
    configure_engine(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:storage:set",
                Some(serde_json::Value::Null),
            ),
            IpcMessage::response("plugin-request-2", "plugin:storage:get", Some("abc".into())),
            IpcMessage::response(
                "plugin-request-3",
                "plugin:request-access",
                Some(serde_json::json!({ "grantId": "grant-1", "path": "C:\\data\\search" })),
            ),
            IpcMessage::response(
                "plugin-request-4",
                "plugin:bind-grant",
                Some(serde_json::json!({ "grantId": "grant-1", "path": "C:\\data\\search" })),
            ),
            IpcMessage::response(
                "plugin-request-5",
                "plugin:grant-fs:exists",
                Some(true.into()),
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
    assert_eq!(outbound[4].method, "plugin:bind-grant");
    assert_eq!(outbound[5].method, "plugin:grant-fs:exists");
    assert_eq!(outbound[6].method, "activate");
}

#[test]
fn delegated_grants_can_list_and_bind_without_requesting_access_again() {
    let config = build_config_with_grants(
        "predelegated-grants",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    const grants = await context.grants.list();
                    if (!grants.includes('grant-1')) {
                        throw new Error('missing grant');
                    }
                    const scoped = await context.grants.bindFsScope('grant-1');
                    await scoped.readFile('child.txt');
                }
            });
        "#,
        vec![DelegatedGrant {
            grant_id: "grant-1".into(),
            path: "C:\\data\\search".into(),
        }],
    );
    configure_engine(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:list-grants",
                Some(serde_json::json!(["grant-1"])),
            ),
            IpcMessage::response(
                "plugin-request-2",
                "plugin:bind-grant",
                Some(serde_json::json!({ "grantId": "grant-1", "path": "C:\\data\\search" })),
            ),
            IpcMessage::response(
                "plugin-request-3",
                "plugin:grant-fs:read-file",
                Some("ok".into()),
            ),
        ],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:list-grants");
    assert_eq!(outbound[2].method, "plugin:bind-grant");
    assert_eq!(outbound[3].method, "plugin:grant-fs:read-file");
    assert_eq!(outbound[4].method, "activate");
}

#[test]
fn non_delegated_grant_binding_surfaces_host_error() {
    let config = build_config(
        "invalid-bind",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    await context.grants.bindFsScope('missing-grant');
                }
            });
        "#,
    );
    configure_engine(
        &config,
        vec![IpcMessage::error_response(
            "plugin-request-1",
            "plugin:bind-grant",
            "PLUGIN_FS_ERROR",
            "grant 'missing-grant' is not delegated to plugin 'acme.search'",
        )],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate dispatch");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:bind-grant");
    assert_eq!(outbound[2].method, "activate");
    assert_eq!(
        outbound[2].error.as_ref().map(|error| error.code.as_str()),
        Some("PLUGIN_RUNTIME_ERROR")
    );
}

#[test]
fn revoked_grant_handles_become_inert_and_emit_revocation_event() {
    let config = build_config_with_grants(
        "grant-revoked",
        r#"
            import { definePlugin } from 'volt:plugin';
            definePlugin({
                async activate(context) {
                    const scoped = await context.grants.bindFsScope('grant-1');
                    context.events.on('grant:revoked', async (payload) => {
                        await context.storage.set('revoked', payload.grantId);
                        try {
                            await scoped.exists('child.txt');
                        } catch (error) {
                            context.log.warn(String(error));
                        }
                    });
                }
            });
        "#,
        vec![DelegatedGrant {
            grant_id: "grant-1".into(),
            path: "C:\\data\\search".into(),
        }],
    );
    configure_engine(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:bind-grant",
                Some(serde_json::json!({ "grantId": "grant-1", "path": "C:\\data\\search" })),
            ),
            IpcMessage::response(
                "plugin-request-2",
                "plugin:subscribe-event",
                Some(true.into()),
            ),
            IpcMessage::response(
                "plugin-request-3",
                "plugin:storage:set",
                Some(serde_json::Value::Null),
            ),
        ],
    );

    let mut engine = PluginEngine::start_with_mock(&config).expect("engine");
    engine
        .dispatch_message(IpcMessage::signal("activate-1", "activate"))
        .expect("activate");
    engine
        .dispatch_message(IpcMessage {
            msg_type: MessageType::Event,
            id: "revoked-1".into(),
            method: "plugin:grant-revoked".into(),
            payload: Some(serde_json::json!({ "grantId": "grant-1" })),
            error: None,
        })
        .expect("grant revoked");

    let outbound = take_outbound();
    assert_eq!(outbound[0].method, "ready");
    assert_eq!(outbound[1].method, "plugin:bind-grant");
    assert_eq!(outbound[2].method, "plugin:subscribe-event");
    assert_eq!(outbound[3].method, "activate");
    assert_eq!(outbound[4].method, "plugin:storage:set");
    assert_eq!(outbound[5].method, "plugin:log");
    assert_eq!(outbound.len(), 6);
}
