use boa_engine::{Context, JsResult, JsValue};

use super::bridge_support::{request_value, request_void, required_string, string_arg};

pub(super) fn storage_get(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    key_request_value("plugin:storage:get", args, context)
}

pub(super) fn storage_set(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let key = required_string(args, 0, context, "storage key")?;
    let value = string_arg(args, 1, context, "storage value")?;
    request_void(
        "plugin:storage:set",
        serde_json::json!({ "key": key, "value": value }),
    )
}

pub(super) fn storage_has(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    key_request_value("plugin:storage:has", args, context)
}

pub(super) fn storage_delete(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let key = required_string(args, 0, context, "storage key")?;
    request_void("plugin:storage:delete", serde_json::json!({ "key": key }))
}

pub(super) fn storage_keys(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    request_value("plugin:storage:keys", serde_json::json!({}), context)
}

fn key_request_value(method: &str, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let key = required_string(args, 0, context, "storage key")?;
    request_value(method, serde_json::json!({ "key": key }), context)
}
