use crate::engine::PluginEngine;
use crate::ipc::{IpcMessage, MessageType};
use crate::runtime_state::take_outbound;

use super::{build_config, configure_engine};

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
    configure_engine(
        &config,
        vec![
            IpcMessage::response(
                "plugin-request-1",
                "plugin:register-command",
                Some(true.into()),
            ),
            IpcMessage::response("plugin-request-2", "plugin:register-ipc", Some(true.into())),
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
