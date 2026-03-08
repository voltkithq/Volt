use boa_engine::{Context, JsError, JsValue};
use serde_json::Value as JsonValue;
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IpcResponse};

pub(super) fn js_error(error: JsError) -> String {
    error.to_string()
}

pub(super) fn js_value_to_json(context: &mut Context, value: JsValue) -> Result<JsonValue, String> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(JsonValue::Null))
        .map_err(js_error)
}

pub(super) fn js_value_to_string(context: &mut Context, value: JsValue) -> Result<String, String> {
    value
        .to_string(context)
        .map(|text| text.to_std_string_escaped())
        .map_err(js_error)
}

pub(super) fn response_for_dispatch_payload(request_id: String, payload: JsonValue) -> IpcResponse {
    let Some(object) = payload.as_object() else {
        return IpcResponse::error_with_code(
            request_id,
            "IPC dispatcher returned invalid payload".to_string(),
            IPC_HANDLER_ERROR_CODE.to_string(),
        );
    };

    let Some(ok) = object.get("ok").and_then(JsonValue::as_bool) else {
        return IpcResponse::error_with_code(
            request_id,
            "IPC response missing 'ok' field".to_string(),
            IPC_HANDLER_ERROR_CODE.to_string(),
        );
    };
    if ok {
        return IpcResponse::success(
            request_id,
            object.get("result").cloned().unwrap_or(JsonValue::Null),
        );
    }

    let error_message = object
        .get("error")
        .and_then(JsonValue::as_str)
        .unwrap_or("IPC handler execution failed")
        .to_string();
    let error_code = object
        .get("errorCode")
        .and_then(JsonValue::as_str)
        .unwrap_or(IPC_HANDLER_ERROR_CODE)
        .to_string();
    if let Some(details) = object.get("errorDetails").cloned() {
        return IpcResponse::error_with_details(request_id, error_message, error_code, details);
    }
    IpcResponse::error_with_code(request_id, error_message, error_code)
}

pub(super) fn extract_ipc_request_id(raw: &str) -> String {
    match serde_json::from_str::<JsonValue>(raw) {
        Ok(value) => extract_ipc_request_id_from_value(&value),
        Err(_) => "unknown".to_string(),
    }
}

pub(super) fn extract_ipc_request_id_from_value(value: &JsonValue) -> String {
    value
        .get("id")
        .and_then(JsonValue::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_for_dispatch_payload_requires_ok_field() {
        let response =
            response_for_dispatch_payload("req-1".to_string(), serde_json::json!({ "result": 1 }));

        assert_eq!(response.id, "req-1");
        assert_eq!(
            response.error.as_deref(),
            Some("IPC response missing 'ok' field")
        );
        assert_eq!(response.error_code.as_deref(), Some(IPC_HANDLER_ERROR_CODE));
    }
}
