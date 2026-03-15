use boa_engine::module::{IntoJsModule, Module};
use boa_engine::native_function::NativeFunction;
use boa_engine::object::JsObject;
use boa_engine::object::builtins::{JsFunction, JsPromise};
use boa_engine::{Context, JsError, JsNativeError, JsResult, JsValue, js_string};

pub fn js_error(
    _module: &'static str,
    _function: &'static str,
    message: impl Into<String>,
) -> JsError {
    JsNativeError::error().with_message(message.into()).into()
}

pub fn format_js_error(error: JsError) -> String {
    error.to_string()
}

pub fn normalize_single_event_name(
    feature_name: &str,
    event_name: String,
    accepted_event_name: &'static str,
    native_event_name: &'static str,
) -> Result<&'static str, String> {
    match event_name.trim() {
        name if name == accepted_event_name => Ok(native_event_name),
        "" => Err(format!("{feature_name} event name must not be empty")),
        other => Err(format!(
            "unsupported {feature_name} event '{other}', only '{accepted_event_name}' is supported"
        )),
    }
}

pub fn bind_native_event_handler(
    context: &mut Context,
    module_name: &'static str,
    api_function: &'static str,
    global_name: &'static str,
    event_type: &str,
    handler: JsFunction,
) -> JsResult<()> {
    let binder = context
        .global_object()
        .get(js_string!(global_name), context)
        .map_err(|error| {
            js_error(
                module_name,
                api_function,
                format!(
                    "native event bridge is unavailable: {}",
                    format_js_error(error)
                ),
            )
        })?;
    let binder = binder.as_callable().ok_or_else(|| {
        js_error(
            module_name,
            api_function,
            "native event bridge is unavailable: binder is not callable",
        )
    })?;

    binder
        .call(
            &JsValue::undefined(),
            &[JsValue::from(js_string!(event_type)), handler.into()],
            context,
        )
        .map(|_| ())
        .map_err(|error| {
            js_error(
                module_name,
                api_function,
                format!(
                    "failed to bind native event handler: {}",
                    format_js_error(error)
                ),
            )
        })
}

pub fn reject_promise(context: &mut Context, message: impl Into<String>) -> JsPromise {
    let message = message.into();
    JsPromise::reject(
        JsError::from_opaque(js_string!(message.as_str()).into()),
        context,
    )
}

pub(crate) trait IntoJsRuntimeValue {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String>;
}

macro_rules! impl_into_js_runtime_value_via_from {
    ($($type:ty),* $(,)?) => {
        $(
            impl IntoJsRuntimeValue for $type {
                fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
                    Ok(JsValue::from(self))
                }
            }
        )*
    };
}

impl_into_js_runtime_value_via_from!((), bool, i32, i64, u32, u64, usize, f64);

impl IntoJsRuntimeValue for JsValue {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(self)
    }
}

impl IntoJsRuntimeValue for JsObject {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(self.into())
    }
}

impl IntoJsRuntimeValue for String {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        Ok(js_string!(self.as_str()).into())
    }
}

impl IntoJsRuntimeValue for Option<String> {
    fn into_js_runtime_value(self, _context: &mut Context) -> Result<JsValue, String> {
        match self {
            Some(value) => Ok(js_string!(value.as_str()).into()),
            None => Ok(JsValue::null()),
        }
    }
}

impl IntoJsRuntimeValue for Vec<String> {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String> {
        json_to_js_value(
            &serde_json::Value::Array(self.into_iter().map(serde_json::Value::String).collect()),
            context,
        )
    }
}

impl IntoJsRuntimeValue for serde_json::Value {
    fn into_js_runtime_value(self, context: &mut Context) -> Result<JsValue, String> {
        json_to_js_value(&self, context)
    }
}

pub fn resolve_promise<V: IntoJsRuntimeValue>(context: &mut Context, value: V) -> JsPromise {
    match value.into_js_runtime_value(context) {
        Ok(value) => JsPromise::resolve(value, context),
        Err(error) => reject_promise(context, error),
    }
}

pub fn promise_from_result<V: IntoJsRuntimeValue>(
    context: &mut Context,
    result: Result<V, String>,
) -> JsPromise {
    match result {
        Ok(value) => resolve_promise(context, value),
        Err(message) => reject_promise(context, message),
    }
}

pub fn json_to_js_value(
    value: &serde_json::Value,
    context: &mut Context,
) -> Result<JsValue, String> {
    JsValue::from_json(value, context).map_err(format_js_error)
}

pub fn value_to_json(value: JsValue, context: &mut Context) -> Result<serde_json::Value, String> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(serde_json::Value::Null))
        .map_err(format_js_error)
}

pub fn promise_from_json_result(
    context: &mut Context,
    result: Result<serde_json::Value, String>,
) -> JsPromise {
    match result {
        Ok(value) => match json_to_js_value(&value, context) {
            Ok(js_value) => resolve_promise(context, js_value),
            Err(error) => reject_promise(context, error),
        },
        Err(error) => reject_promise(context, error),
    }
}

pub fn native_function_module(
    context: &mut Context,
    exports: Vec<(&'static str, NativeFunction)>,
) -> Module {
    exports
        .into_iter()
        .map(|(name, function)| (js_string!(name), function))
        .collect::<Vec<_>>()
        .into_js_module(context)
}
