use std::rc::Rc;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsValue, js_string};
use serde_json::Value as JsonValue;

use super::NATIVE_EVENT_DISPATCH_SAFE_GLOBAL;

pub(super) async fn dispatch_native_event_handler(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    event_type: &str,
    payload: &JsonValue,
) -> Result<(), String> {
    let dispatch = context
        .global_object()
        .get(js_string!(NATIVE_EVENT_DISPATCH_SAFE_GLOBAL), context)
        .map_err(|error| format!("native event bridge is not initialized: {error}"))?;
    let dispatch = dispatch.as_callable().ok_or_else(|| {
        "native event bridge is not initialized: dispatcher is not callable".to_string()
    })?;

    let payload_value = JsValue::from_json(payload, context)
        .map_err(|error| format!("failed to parse native event payload: {error}"))?;
    let promise_value = dispatch
        .call(
            &JsValue::undefined(),
            &[JsValue::from(js_string!(event_type)), payload_value],
            context,
        )
        .map_err(|error| format!("failed to invoke native event dispatcher: {error}"))?;
    let promise = promise_from_value(promise_value)?;

    let mut iterations = 0;
    loop {
        super::bootstrap::run_jobs(context, job_executor).await?;
        iterations += 1;

        match promise.state() {
            PromiseState::Pending if iterations < super::ipc::MAX_JOB_ITERATIONS => continue,
            PromiseState::Pending => {
                tracing::warn!(
                    event_type = %event_type,
                    "promise did not settle after {iterations} job iterations"
                );
                return Err("native event promise did not settle".to_string());
            }
            PromiseState::Fulfilled(_) => return Ok(()),
            PromiseState::Rejected(error) => {
                return Err(format!(
                    "native event handler promise rejected: {}",
                    super::serde_support::js_value_to_string(context, error)?
                ));
            }
        }
    }
}

fn promise_from_value(value: JsValue) -> Result<JsPromise, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "native event dispatcher did not return a Promise".to_string())?;

    JsPromise::from_object(object).map_err(super::serde_support::js_error)
}
