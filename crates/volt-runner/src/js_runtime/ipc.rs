use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsValue, js_string};
use serde_json::Value as JsonValue;
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IPC_MAX_REQUEST_BYTES, IpcRequest, IpcResponse};

use super::IPC_DISPATCH_SAFE_GLOBAL;
const IPC_PROTOTYPE_CHECK_MAX_DEPTH: usize = 64;

#[derive(Default)]
pub(super) struct IpcRuntimeState {
    requests: VecDeque<Instant>,
}

impl IpcRuntimeState {
    fn check_rate_limit(&mut self) -> Result<(), String> {
        let now = Instant::now();
        while let Some(oldest) = self.requests.front() {
            if now.duration_since(*oldest) < super::IPC_RATE_LIMIT_WINDOW {
                break;
            }
            self.requests.pop_front();
        }

        if self.requests.len() >= super::IPC_RATE_LIMIT_MAX_REQUESTS {
            return Err(format!(
                "rate limit exceeded: {} requests/second",
                super::IPC_RATE_LIMIT_MAX_REQUESTS
            ));
        }

        self.requests.push_back(now);
        Ok(())
    }
}

pub(super) async fn dispatch_ipc_request(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    ipc_state: &mut IpcRuntimeState,
    raw: &str,
    timeout: Duration,
    deadline: Instant,
) -> IpcResponse {
    let request = match parse_ipc_request(raw, ipc_state) {
        Ok(request) => request,
        Err(response) => return *response,
    };

    if let Some(result) =
        crate::modules::volt_bench::dispatch_native_fast_path(&request.method, request.args.clone())
    {
        return match result {
            Ok(payload) => IpcResponse::success(request.id, payload),
            Err(message) => IpcResponse::error_with_code(
                request.id,
                message,
                IPC_HANDLER_ERROR_CODE.to_string(),
            ),
        };
    }

    let dispatch_timeout = super::normalize_ipc_timeout(timeout);
    let remaining_timeout = deadline.saturating_duration_since(Instant::now());
    let queue_delay = dispatch_timeout.saturating_sub(remaining_timeout);
    if remaining_timeout.is_zero() {
        return super::serde_support::ipc_timeout_response(
            request.id,
            request.method,
            dispatch_timeout,
            queue_delay,
        );
    }
    let handler_timeout = super::ipc_inner_timeout_budget(remaining_timeout);

    let request_id = request.id.clone();
    let dispatch_result = tokio::time::timeout(
        handler_timeout,
        dispatch_ipc_handler(context, job_executor, &request.method, &request.args),
    )
    .await;

    match dispatch_result {
        Ok(Ok(response)) => {
            super::serde_support::response_for_dispatch_payload(request_id, response)
        }
        Ok(Err(message)) => {
            IpcResponse::error_with_code(request_id, message, IPC_HANDLER_ERROR_CODE.to_string())
        }
        Err(_) => super::serde_support::ipc_timeout_response(
            request_id,
            request.method,
            dispatch_timeout,
            queue_delay,
        ),
    }
}

fn parse_ipc_request(
    raw: &str,
    ipc_state: &mut IpcRuntimeState,
) -> Result<IpcRequest, Box<IpcResponse>> {
    if raw.len() > IPC_MAX_REQUEST_BYTES {
        return Err(Box::new(IpcResponse::error_with_details(
            super::serde_support::extract_ipc_request_id(raw),
            format!(
                "IPC payload too large ({} bytes > {} bytes)",
                raw.len(),
                IPC_MAX_REQUEST_BYTES
            ),
            "IPC_PAYLOAD_TOO_LARGE".to_string(),
            serde_json::json!({
                "payloadBytes": raw.len(),
                "maxPayloadBytes": IPC_MAX_REQUEST_BYTES
            }),
        )));
    }

    if let Err(message) = ipc_state.check_rate_limit() {
        return Err(Box::new(IpcResponse::error_with_code(
            super::serde_support::extract_ipc_request_id(raw),
            message,
            IPC_HANDLER_ERROR_CODE.to_string(),
        )));
    }

    let parsed: JsonValue = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            return Err(Box::new(IpcResponse::error_with_code(
                super::serde_support::extract_ipc_request_id(raw),
                format!("invalid JSON message: {error}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            )));
        }
    };
    let request_id = super::serde_support::extract_ipc_request_id_from_value(&parsed);

    if let Err(message) = validate_prototype_pollution(&parsed, 0) {
        return Err(Box::new(IpcResponse::error_with_code(
            request_id.clone(),
            message,
            IPC_HANDLER_ERROR_CODE.to_string(),
        )));
    }

    let request: IpcRequest = serde_json::from_value(parsed).map_err(|error| {
        Box::new(IpcResponse::error_with_code(
            request_id,
            format!("invalid IPC request: {error}"),
            IPC_HANDLER_ERROR_CODE.to_string(),
        ))
    })?;

    if request.method.trim().is_empty() {
        return Err(Box::new(IpcResponse::error_with_code(
            request.id,
            "invalid IPC request: method must not be empty".to_string(),
            IPC_HANDLER_ERROR_CODE.to_string(),
        )));
    }

    Ok(request)
}

fn validate_prototype_pollution(value: &JsonValue, depth: usize) -> Result<(), String> {
    if depth > IPC_PROTOTYPE_CHECK_MAX_DEPTH {
        return Err(format!(
            "payload nesting exceeds max depth ({depth} > {IPC_PROTOTYPE_CHECK_MAX_DEPTH})"
        ));
    }

    match value {
        JsonValue::Object(map) => {
            for key in map.keys() {
                if key == "__proto__" || key == "constructor" || key == "prototype" {
                    return Err("prototype pollution attempt detected".to_string());
                }
            }
            for nested in map.values() {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        JsonValue::Array(array) => {
            for nested in array {
                validate_prototype_pollution(nested, depth + 1)?;
            }
        }
        _ => {}
    }

    Ok(())
}

pub(super) const MAX_JOB_ITERATIONS: usize = 1000;

async fn dispatch_ipc_handler(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    method: &str,
    args: &JsonValue,
) -> Result<JsonValue, String> {
    let dispatch = context
        .global_object()
        .get(js_string!(IPC_DISPATCH_SAFE_GLOBAL), context)
        .map_err(|error| format!("IPC runtime is not initialized: {error}"))?;
    let dispatch = dispatch
        .as_callable()
        .ok_or_else(|| "IPC runtime is not initialized: dispatcher is not callable".to_string())?;

    let args_value = JsValue::from_json(args, context)
        .map_err(|error| format!("failed to parse IPC arguments: {error}"))?;
    let promise_value = dispatch
        .call(
            &JsValue::undefined(),
            &[JsValue::from(js_string!(method)), args_value],
            context,
        )
        .map_err(|error| format!("failed to invoke IPC dispatcher: {error}"))?;
    if !promise_value.is_object() {
        return Err("IPC dispatcher did not return an object".to_string());
    }
    let promise = promise_from_value(promise_value)?;

    // Async JS handlers with multiple `await` points need multiple rounds
    // of job execution. Each `await` in the handler creates a continuation
    // job that must be drained before the next `await` can proceed.
    let mut iterations = 0;
    loop {
        // Settle any completed async dialogs before running JS jobs,
        // so their resolve/reject calls become available as new jobs.
        crate::modules::dialog_async::settle_pending_dialogs(context);
        super::bootstrap::run_jobs(context, job_executor).await?;
        iterations += 1;

        match promise.state() {
            PromiseState::Pending if crate::modules::dialog_async::has_pending_dialogs() => {
                // A dialog is pending on another thread — yield briefly and
                // retry rather than burning through the iteration budget.
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }
            PromiseState::Pending if iterations < MAX_JOB_ITERATIONS => continue,
            PromiseState::Pending => {
                tracing::warn!(method = %method, "promise did not settle after {iterations} job iterations");
                return Err("IPC promise did not settle".to_string());
            }
            PromiseState::Fulfilled(payload) => {
                return super::serde_support::js_value_to_json(context, payload)
                    .map_err(|error| format!("failed to decode IPC response payload: {error}"));
            }
            PromiseState::Rejected(error) => {
                return Err(format!(
                    "failed to resolve IPC promise: {}",
                    super::serde_support::js_value_to_string(context, error)?
                ));
            }
        }
    }
}

fn promise_from_value(value: JsValue) -> Result<JsPromise, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "IPC dispatcher did not return a Promise".to_string())?;

    JsPromise::from_object(object).map_err(super::serde_support::js_error)
}
