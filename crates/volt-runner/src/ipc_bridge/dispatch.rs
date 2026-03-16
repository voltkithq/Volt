use std::time::Duration;

use serde_json::Value as JsonValue;
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IPC_HANDLER_TIMEOUT_CODE, IpcRequest, IpcResponse};

use super::request_validation::{extract_request_id_from_value, validate_prototype_pollution};
use crate::js_runtime_pool::JsRuntimePoolClient;
use crate::modules::volt_bench;
use crate::plugin_manager::PluginManager;

pub(super) fn try_dispatch_native_fast_path(raw: &str) -> Option<IpcResponse> {
    let parsed: JsonValue = serde_json::from_str(raw).ok()?;
    let request_id = extract_request_id_from_value(&parsed);

    if let Err(message) = validate_prototype_pollution(&parsed, 0) {
        return Some(IpcResponse::error_with_code(
            request_id,
            message,
            IPC_HANDLER_ERROR_CODE.to_string(),
        ));
    }

    let request: IpcRequest = match serde_json::from_value(parsed) {
        Ok(request) => request,
        Err(error) => {
            return Some(IpcResponse::error_with_code(
                request_id,
                format!("invalid IPC request: {error}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            ));
        }
    };

    if request.method == "__volt_internal:csp-violation" {
        let blocked = request
            .args
            .get("blockedURI")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        let directive = request
            .args
            .get("violatedDirective")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown");
        tracing::warn!(
            blocked_uri = blocked,
            directive = directive,
            "CSP violation: {blocked} blocked by \"{directive}\" directive. \
             Review CSP settings or check the resource URL."
        );
        return Some(IpcResponse::success(request.id, serde_json::json!(null)));
    }

    volt_bench::dispatch_native_fast_path(&request.method, request.args).map(
        |result| match result {
            Ok(payload) => IpcResponse::success(request.id, payload),
            Err(error) => {
                IpcResponse::error_with_code(request.id, error, IPC_HANDLER_ERROR_CODE.to_string())
            }
        },
    )
}

pub(super) fn dispatch_ipc_task(
    runtime_client: &JsRuntimePoolClient,
    plugin_manager: Option<&PluginManager>,
    raw: &str,
    request_id: &str,
    timeout: Duration,
) -> IpcResponse {
    // Check rate limit BEFORE executing any work (including native fast paths)
    // so that rate-limited requests are rejected without performing computation.
    if let Err(error) = runtime_client.check_ipc_rate_limit() {
        return IpcResponse::error_with_code(
            request_id.to_string(),
            error,
            IPC_HANDLER_ERROR_CODE.to_string(),
        );
    }

    if let Some(response) = try_dispatch_native_fast_path(raw) {
        return response;
    }

    if let Some(plugin_manager) = plugin_manager
        && let Some(response) = try_dispatch_plugin_route(plugin_manager, raw, timeout)
    {
        return response;
    }

    runtime_client
        .dispatch_ipc_message(raw, timeout)
        .unwrap_or_else(|error| {
            if error.contains("timed out after") {
                let method = crate::js_runtime::serde_support::extract_ipc_method(raw);
                IpcResponse::error_with_details(
                    request_id.to_string(),
                    format!("IPC bridge failure: {error}"),
                    IPC_HANDLER_TIMEOUT_CODE.to_string(),
                    serde_json::json!({
                        "timeoutMs": timeout.as_millis(),
                        "method": method
                    }),
                )
            } else {
                IpcResponse::error_with_code(
                    request_id.to_string(),
                    format!("IPC bridge failure: {error}"),
                    IPC_HANDLER_ERROR_CODE.to_string(),
                )
            }
        })
}

fn try_dispatch_plugin_route(
    plugin_manager: &PluginManager,
    raw: &str,
    timeout: Duration,
) -> Option<IpcResponse> {
    let parsed: JsonValue = serde_json::from_str(raw).ok()?;
    if let Err(message) = validate_prototype_pollution(&parsed, 0) {
        return Some(IpcResponse::error_with_code(
            extract_request_id_from_value(&parsed),
            message,
            IPC_HANDLER_ERROR_CODE.to_string(),
        ));
    }
    let request: IpcRequest = serde_json::from_value(parsed).ok()?;
    plugin_manager.handle_ipc_request(&request, timeout)
}

