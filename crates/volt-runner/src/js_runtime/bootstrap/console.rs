use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsResult, JsValue, js_string};

fn format_console_args(args: &[JsValue], context: &mut Context) -> String {
    args.iter()
        .map(|value| {
            super::super::serde_support::js_value_to_string(context, value.clone())
                .unwrap_or_else(|error| format!("<unprintable: {error}>"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn console_log(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    println!("{message}");
    Ok(JsValue::undefined())
}

fn console_info(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    println!("{message}");
    Ok(JsValue::undefined())
}

fn console_warn(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    eprintln!("{message}");
    Ok(JsValue::undefined())
}

fn console_error(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    eprintln!("{message}");
    Ok(JsValue::undefined())
}

pub(super) fn register_console(context: &mut Context) -> JsResult<()> {
    let console = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(console_log),
            js_string!("log"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_info),
            js_string!("info"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_warn),
            js_string!("warn"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_error),
            js_string!("error"),
            1,
        )
        .build();

    context.register_global_property(js_string!("console"), console, Attribute::all())
}
