use std::time::Duration;

use serde_json::Value as JsonValue;
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IpcResponse};

use super::JsRuntimePoolClient;

impl JsRuntimePoolClient {
    fn stateful_client(&self) -> Result<&crate::js_runtime::JsRuntimeClient, String> {
        self.clients
            .first()
            .ok_or_else(|| "js runtime pool is empty".to_string())
    }

    pub fn eval_i64(&self, script: &str) -> Result<i64, String> {
        self.stateful_client()?.eval_i64(script)
    }

    pub fn load_backend_bundle(&self, script: &str) -> Result<(), String> {
        self.stateful_client()?
            .load_backend_bundle(script)
            .map_err(|error| format!("failed to load backend bundle in runtime 0: {error}"))
    }

    pub fn dispatch_ipc_message(
        &self,
        raw: &str,
        timeout: Duration,
    ) -> Result<IpcResponse, String> {
        if let Err(message) = self.check_ipc_rate_limit() {
            return Ok(IpcResponse::error_with_code(
                extract_request_id(raw),
                message,
                IPC_HANDLER_ERROR_CODE.to_string(),
            ));
        }
        self.stateful_client()?.dispatch_ipc_message(raw, timeout)
    }

    pub fn dispatch_native_event(
        &self,
        event_type: &str,
        payload: JsonValue,
    ) -> Result<(), String> {
        self.stateful_client()?
            .dispatch_native_event(event_type, payload)
    }

    fn check_ipc_rate_limit(&self) -> Result<(), String> {
        let mut limiter = self
            .ipc_rate_limiter
            .lock()
            .map_err(|_| "IPC rate limiter state is unavailable".to_string())?;
        limiter.check_rate_limit()
    }
}

fn extract_request_id(raw: &str) -> String {
    serde_json::from_str::<JsonValue>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(JsonValue::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}
