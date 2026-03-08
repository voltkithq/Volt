use boa_engine::builtins::promise::PromiseState;
use boa_engine::object::builtins::{JsFunction, JsPromise};
use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use serde_json::{Value, json};

use crate::modules::{
    format_js_error, native_function_module, promise_from_json_result, promise_from_result,
    value_to_json,
};

use super::security::ensure_database_permission;
use super::sql::{execute_sql, parse_sql_params, query_one_sql, query_sql, run_transaction};
use super::state::{close_database, open_database};

const MAX_JOB_ITERATIONS: usize = 1000;

fn open(path: String, context: &mut Context) -> JsValue {
    promise_from_result(context, open_database(&path)).into()
}

fn close(context: &mut Context) -> JsValue {
    promise_from_result(context, close_database()).into()
}

fn execute(sql: String, params: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        ensure_database_permission()?;
        let parsed_params = parse_sql_params(params, context)?;
        let rows_affected = execute_sql(&sql, &parsed_params)?;
        Ok(json!({ "rowsAffected": rows_affected }))
    })();

    promise_from_json_result(context, result).into()
}

fn query(sql: String, params: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        ensure_database_permission()?;
        let parsed_params = parse_sql_params(params, context)?;
        let rows = query_sql(&sql, &parsed_params)?;
        Ok(Value::Array(rows))
    })();

    promise_from_json_result(context, result).into()
}

fn query_one(sql: String, params: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        ensure_database_permission()?;
        let parsed_params = parse_sql_params(params, context)?;
        Ok(query_one_sql(&sql, &parsed_params)?.unwrap_or(Value::Null))
    })();

    promise_from_json_result(context, result).into()
}

fn resolve_callback_result(
    callback_result: JsValue,
    context: &mut Context,
) -> Result<Value, String> {
    let Some(promise) = callback_result.as_promise() else {
        return value_to_json(callback_result, context);
    };

    wait_for_promise(&promise, context)
}

fn wait_for_promise(promise: &JsPromise, context: &mut Context) -> Result<Value, String> {
    let mut iterations = 0;
    loop {
        context.run_jobs().map_err(format_js_error)?;
        iterations += 1;

        match promise.state() {
            PromiseState::Fulfilled(value) => return value_to_json(value, context),
            PromiseState::Rejected(error) => {
                let message = error
                    .to_string(context)
                    .map(|value| value.to_std_string_escaped())
                    .unwrap_or_else(|_| error.display().to_string());
                return Err(format!("transaction callback rejected: {message}"));
            }
            PromiseState::Pending if iterations < MAX_JOB_ITERATIONS => continue,
            PromiseState::Pending => {
                return Err(
                    "transaction callback promise did not settle within iteration limit"
                        .to_string(),
                );
            }
        }
    }
}

fn transaction(callback: JsFunction, context: &mut Context) -> JsValue {
    let result = (|| {
        ensure_database_permission()?;
        run_transaction(|| {
            let callback_result = callback
                .clone()
                .call(&JsValue::undefined(), &[], context)
                .map_err(|error| {
                    format!(
                        "failed to execute transaction callback: {}",
                        format_js_error(error)
                    )
                })?;
            resolve_callback_result(callback_result, context)
        })
    })();

    promise_from_json_result(context, result).into()
}

pub fn build_module(context: &mut Context) -> Module {
    let open = open.into_js_function_copied(context);
    let close = close.into_js_function_copied(context);
    let execute = execute.into_js_function_copied(context);
    let query = query.into_js_function_copied(context);
    let query_one = query_one.into_js_function_copied(context);
    let transaction = transaction.into_js_function_copied(context);
    let exports = vec![
        ("open", open),
        ("close", close),
        ("execute", execute),
        ("query", query),
        ("queryOne", query_one),
        ("transaction", transaction),
    ];
    native_function_module(context, exports)
}
