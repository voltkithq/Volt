mod analytics;
#[cfg(test)]
mod tests;
mod workflow;

use super::{native_function_module, promise_from_json_result, value_to_json};
use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use serde::de::DeserializeOwned;
use serde_json::Value;

use self::analytics::{
    AnalyticsBenchmarkOptions, AnalyticsProfileOptions, analytics_profile_json,
    run_analytics_benchmark_json,
};
use self::workflow::{WorkflowBenchmarkOptions, run_workflow_benchmark_json};

pub const DATA_PROFILE_CHANNEL: &str = "volt:native:data.profile";
pub const DATA_QUERY_CHANNEL: &str = "volt:native:data.query";
pub const WORKFLOW_RUN_CHANNEL: &str = "volt:native:workflow.run";

fn analytics_profile(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = deserialize_options::<AnalyticsProfileOptions>(options, context)
        .and_then(analytics_profile_json);
    promise_from_json_result(context, result).into()
}

fn run_analytics_benchmark(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = deserialize_options::<AnalyticsBenchmarkOptions>(options, context)
        .and_then(run_analytics_benchmark_json);
    promise_from_json_result(context, result).into()
}

fn run_workflow_benchmark(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = deserialize_options::<WorkflowBenchmarkOptions>(options, context)
        .and_then(run_workflow_benchmark_json);
    promise_from_json_result(context, result).into()
}

fn deserialize_options<T: DeserializeOwned + Default>(
    options: Option<JsValue>,
    context: &mut Context,
) -> Result<T, String> {
    let Some(options) = options else {
        return Ok(T::default());
    };
    let json = value_to_json(options, context)?;
    if json.is_null() {
        return Ok(T::default());
    }
    serde_json::from_value::<T>(json)
        .map_err(|error| format!("invalid volt:bench options: {error}"))
}

fn deserialize_json_options<T: DeserializeOwned + Default>(json: Value) -> Result<T, String> {
    if json.is_null() {
        return Ok(T::default());
    }
    serde_json::from_value::<T>(json)
        .map_err(|error| format!("invalid volt:bench options: {error}"))
}

pub fn dispatch_native_fast_path(method: &str, args: Value) -> Option<Result<Value, String>> {
    match method {
        DATA_PROFILE_CHANNEL => Some(
            deserialize_json_options::<AnalyticsProfileOptions>(args)
                .and_then(analytics_profile_json),
        ),
        DATA_QUERY_CHANNEL => Some(
            deserialize_json_options::<AnalyticsBenchmarkOptions>(args)
                .and_then(run_analytics_benchmark_json),
        ),
        WORKFLOW_RUN_CHANNEL => Some(
            deserialize_json_options::<WorkflowBenchmarkOptions>(args)
                .and_then(run_workflow_benchmark_json),
        ),
        _ => None,
    }
}

pub fn build_module(context: &mut Context) -> Module {
    let analytics_profile = analytics_profile.into_js_function_copied(context);
    let run_analytics_benchmark = run_analytics_benchmark.into_js_function_copied(context);
    let run_workflow_benchmark = run_workflow_benchmark.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("analyticsProfile", analytics_profile),
            ("runAnalyticsBenchmark", run_analytics_benchmark),
            ("runWorkflowBenchmark", run_workflow_benchmark),
        ],
    )
}
