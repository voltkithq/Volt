use std::path::Path;
use std::rc::Rc;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::module::Module;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{Context, JsValue, Source};

use super::serde_support;

fn as_i64(value: JsValue) -> Result<i64, String> {
    let number = value
        .as_number()
        .ok_or_else(|| "expected JavaScript number result".to_string())?;
    if !number.is_finite() {
        return Err("expected finite JavaScript number result".to_string());
    }
    if number.fract() != 0.0 {
        return Err(format!(
            "expected integer JavaScript number result, got {number}"
        ));
    }
    if number < i64::MIN as f64 || number > i64::MAX as f64 {
        return Err(format!(
            "JavaScript number result is out of range for i64: {number}"
        ));
    }
    Ok(number as i64)
}

#[cfg(test)]
fn as_bool(value: JsValue) -> Result<bool, String> {
    value
        .as_boolean()
        .ok_or_else(|| "expected JavaScript boolean result".to_string())
}

#[cfg(test)]
fn as_string(context: &mut Context, value: JsValue) -> Result<String, String> {
    serde_support::js_value_to_string(context, value)
}

#[cfg(test)]
fn as_promise(value: JsValue) -> Result<JsPromise, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "expected JavaScript Promise result".to_string())?;
    JsPromise::from_object(object).map_err(serde_support::js_error)
}

async fn settle_promise(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    promise: JsPromise,
) -> Result<JsValue, String> {
    let mut iterations = 0;
    loop {
        super::bootstrap::run_jobs(context, job_executor).await?;
        iterations += 1;

        match promise.state() {
            PromiseState::Pending if iterations < super::ipc::MAX_JOB_ITERATIONS => continue,
            PromiseState::Pending => {
                return Err("JavaScript promise did not settle".to_string());
            }
            PromiseState::Fulfilled(value) => return Ok(value),
            PromiseState::Rejected(value) => {
                return Err(serde_support::js_value_to_string(context, value)?);
            }
        }
    }
}

pub(super) async fn eval_backend_bundle(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    script: &str,
) -> Result<(), String> {
    if script.trim().is_empty() {
        return Ok(());
    }

    let source = Source::from_bytes(script).with_path(Path::new("volt-backend-entry.mjs"));
    let module = Module::parse(source, None, context).map_err(serde_support::js_error)?;
    let promise = module.load_link_evaluate(context);
    let _ = settle_promise(context, job_executor, promise).await?;
    Ok(())
}

pub(super) async fn eval_i64(context: &mut Context, script: &str) -> Result<i64, String> {
    context
        .eval(Source::from_bytes(script))
        .map_err(serde_support::js_error)
        .and_then(as_i64)
}

#[cfg(test)]
pub(super) async fn eval_bool(context: &mut Context, script: &str) -> Result<bool, String> {
    context
        .eval(Source::from_bytes(script))
        .map_err(serde_support::js_error)
        .and_then(as_bool)
}

#[cfg(test)]
pub(super) async fn eval_string(context: &mut Context, script: &str) -> Result<String, String> {
    let value = context
        .eval(Source::from_bytes(script))
        .map_err(serde_support::js_error)?;
    as_string(context, value)
}

#[cfg(test)]
pub(super) async fn eval_unit(context: &mut Context, script: &str) -> Result<(), String> {
    context
        .eval(Source::from_bytes(script))
        .map(|_| ())
        .map_err(serde_support::js_error)
}

#[cfg(test)]
pub(super) async fn eval_promise_i64(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    script: &str,
) -> Result<i64, String> {
    let value = context
        .eval(Source::from_bytes(script))
        .map_err(serde_support::js_error)?;
    let promise = as_promise(value)?;
    let value = settle_promise(context, job_executor, promise).await?;
    as_i64(value)
}

#[cfg(test)]
pub(super) async fn eval_promise_string(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    script: &str,
) -> Result<String, String> {
    let value = context
        .eval(Source::from_bytes(script))
        .map_err(serde_support::js_error)?;
    let promise = as_promise(value)?;
    let value = settle_promise(context, job_executor, promise).await?;
    as_string(context, value)
}
