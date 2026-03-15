use boa_engine::{Context, JsNativeError, JsResult, JsValue};

use crate::runtime_state;

use super::bridge_support::{json_arg, json_response, request_void, required_string};

pub(super) fn manifest(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let manifest =
        runtime_state::manifest().map_err(|error| JsNativeError::error().with_message(error))?;
    json_response(manifest, context)
}

pub(super) fn send_log(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let level = required_string(args, 0, context, "log level")?;
    let message = required_string(args, 1, context, "log message")?;
    runtime_state::send_event(
        "plugin:log",
        serde_json::json!({
            "level": level,
            "message": message,
            "pluginId": runtime_state::plugin_id().unwrap_or_default(),
        }),
    )
    .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}

pub(super) fn register_command(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:register-command", "id", args, context)
}

pub(super) fn unregister_command(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:unregister-command", "id", args, context)
}

pub(super) fn subscribe_event(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:subscribe-event", "event", args, context)
}

pub(super) fn unsubscribe_event(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:unsubscribe-event", "event", args, context)
}

pub(super) fn emit_event(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let event = required_string(args, 0, context, "event name")?;
    let data = json_arg(args.get(1).cloned().unwrap_or(JsValue::null()), context)?;
    request_void(
        "plugin:emit-event",
        serde_json::json!({ "event": event, "data": data }),
    )
}

pub(super) fn register_ipc(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:register-ipc", "channel", args, context)
}

pub(super) fn unregister_ipc(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:unregister-ipc", "channel", args, context)
}

fn register_simple(
    method: &str,
    key: &str,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = required_string(args, 0, context, key)?;
    request_void(method, serde_json::json!({ key: value }))
}
