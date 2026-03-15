use boa_engine::{Context, JsNativeError, JsResult, JsValue};

use crate::runtime_state;

pub(super) fn required_string(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
    label: &str,
) -> JsResult<String> {
    let value = args.get(index).cloned().unwrap_or(JsValue::undefined());
    let string = value
        .to_string(context)
        .map_err(|error| JsNativeError::error().with_message(error.to_string()))?;
    let trimmed = string.to_std_string_escaped().trim().to_string();
    if trimmed.is_empty() {
        return Err(JsNativeError::error()
            .with_message(format!("{label} must not be empty"))
            .into());
    }
    Ok(trimmed)
}

pub(super) fn string_arg(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
    label: &str,
) -> JsResult<String> {
    let value = args.get(index).cloned().unwrap_or(JsValue::undefined());
    let string = value
        .to_string(context)
        .map_err(|error| JsNativeError::error().with_message(error.to_string()))?
        .to_std_string_escaped();
    if string.is_empty() {
        return Err(JsNativeError::error()
            .with_message(format!("{label} must not be empty"))
            .into());
    }
    Ok(string)
}

pub(super) fn json_arg(value: JsValue, context: &mut Context) -> JsResult<serde_json::Value> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(serde_json::Value::Null))
        .map_err(|error| {
            JsNativeError::error()
                .with_message(error.to_string())
                .into()
        })
}

pub(super) fn json_response(value: serde_json::Value, context: &mut Context) -> JsResult<JsValue> {
    JsValue::from_json(&value, context).map_err(|error| {
        JsNativeError::error()
            .with_message(error.to_string())
            .into()
    })
}

pub(super) fn request_value(
    method: &str,
    payload: serde_json::Value,
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = runtime_state::send_request(method, payload)
        .map_err(|error| JsNativeError::error().with_message(error))?;
    json_response(value, context)
}

pub(super) fn request_void(method: &str, payload: serde_json::Value) -> JsResult<JsValue> {
    runtime_state::send_request(method, payload)
        .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}
