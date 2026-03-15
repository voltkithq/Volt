use boa_engine::{Context, JsNativeError, JsResult, JsValue};

use crate::runtime_state;

use super::bridge_support::{json_arg, json_response};

pub(super) fn delegated_grants(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let grants = runtime_state::delegated_grants()
        .map_err(|error| JsNativeError::error().with_message(error))?;
    json_response(serde_json::json!(grants), context)
}

pub(super) fn request_access(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let options = args.first().cloned().unwrap_or(JsValue::undefined());
    let options = if options.is_undefined() {
        serde_json::json!({})
    } else {
        json_arg(options, context)?
    };
    super::bridge_support::request_value("plugin:request-access", options, context)
}

pub(super) fn bind_grant(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let grant_id = super::bridge_support::required_string(args, 0, context, "grant id")?;
    super::bridge_support::request_value(
        "plugin:bind-grant",
        serde_json::json!({ "grantId": grant_id }),
        context,
    )
}

pub(super) fn list_grants(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    super::bridge_support::request_value("plugin:list-grants", serde_json::json!({}), context)
}
