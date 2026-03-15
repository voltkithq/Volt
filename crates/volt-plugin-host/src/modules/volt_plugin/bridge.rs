use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsNativeError, JsResult, JsValue, js_string};

use crate::runtime_state;

pub fn register_native_bridge(context: &mut Context) -> JsResult<()> {
    let bridge = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(manifest),
            js_string!("manifest"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(send_log),
            js_string!("sendLog"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(register_command),
            js_string!("registerCommand"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(unregister_command),
            js_string!("unregisterCommand"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(subscribe_event),
            js_string!("subscribeEvent"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(unsubscribe_event),
            js_string!("unsubscribeEvent"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(emit_event),
            js_string!("emitEvent"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(register_ipc),
            js_string!("registerIpc"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(unregister_ipc),
            js_string!("unregisterIpc"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_read_file),
            js_string!("fsReadFile"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_write_file),
            js_string!("fsWriteFile"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_read_dir),
            js_string!("fsReadDir"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_stat),
            js_string!("fsStat"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_exists),
            js_string!("fsExists"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_mkdir),
            js_string!("fsMkdir"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(fs_remove),
            js_string!("fsRemove"),
            1,
        )
        .build();

    context.register_global_property(
        js_string!("__volt_plugin_native__"),
        bridge,
        Attribute::all(),
    )
}

fn manifest(_this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let manifest =
        runtime_state::manifest().map_err(|error| JsNativeError::error().with_message(error))?;
    JsValue::from_json(&manifest, context).map_err(|error| {
        JsNativeError::error()
            .with_message(error.to_string())
            .into()
    })
}

fn send_log(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let level = required_string(args, 0, context, "log level")?;
    let message = required_string(args, 1, context, "log message")?;
    runtime_state::send_event(
        "plugin:log",
        serde_json::json!({
            "level": level,
            "message": message,
            "pluginId": runtime_state::plugin_id().unwrap_or_default(),
        }),
    )
    .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}

fn register_command(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    register_simple("plugin:register-command", "id", args, context)
}

fn unregister_command(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:unregister-command", "id", args, context)
}

fn subscribe_event(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    register_simple("plugin:subscribe-event", "event", args, context)
}

fn unsubscribe_event(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    register_simple("plugin:unsubscribe-event", "event", args, context)
}

fn emit_event(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let event = required_string(args, 0, context, "event name")?;
    let data = json_arg(args.get(1).cloned().unwrap_or(JsValue::null()), context)?;
    runtime_state::send_request(
        "plugin:emit-event",
        serde_json::json!({ "event": event, "data": data }),
    )
    .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}

fn register_ipc(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    register_simple("plugin:register-ipc", "channel", args, context)
}

fn unregister_ipc(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    register_simple("plugin:unregister-ipc", "channel", args, context)
}

fn fs_read_file(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:read-file", args, context)
}

fn fs_write_file(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = required_string(args, 0, context, "path")?;
    let data = required_string(args, 1, context, "file contents")?;
    runtime_state::send_request(
        "plugin:fs:write-file",
        serde_json::json!({ "path": path, "data": data }),
    )
    .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}

fn fs_read_dir(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:read-dir", args, context)
}

fn fs_stat(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:stat", args, context)
}

fn fs_exists(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:exists", args, context)
}

fn fs_mkdir(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:mkdir", args, context)
}

fn fs_remove(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    fs_request("plugin:fs:remove", args, context)
}

fn register_simple(
    method: &str,
    key: &str,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = required_string(args, 0, context, key)?;
    runtime_state::send_request(method, serde_json::json!({ key: value }))
        .map_err(|error| JsNativeError::error().with_message(error))?;
    Ok(JsValue::undefined())
}

fn fs_request(method: &str, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = required_string(args, 0, context, "path")?;
    let value = runtime_state::send_request(method, serde_json::json!({ "path": path }))
        .map_err(|error| JsNativeError::error().with_message(error))?;
    JsValue::from_json(&value, context).map_err(|error| {
        JsNativeError::error()
            .with_message(error.to_string())
            .into()
    })
}

fn required_string(
    args: &[JsValue],
    index: usize,
    context: &mut Context,
    label: &str,
) -> JsResult<String> {
    let value = args.get(index).cloned().unwrap_or(JsValue::undefined());
    let string = value
        .to_string(context)
        .map_err(|error| JsNativeError::error().with_message(error.to_string()))?;
    let string = string.to_std_string_escaped();
    let trimmed = string.trim();
    if trimmed.is_empty() {
        return Err(JsNativeError::error()
            .with_message(format!("{label} must not be empty"))
            .into());
    }
    Ok(trimmed.to_string())
}

fn json_arg(value: JsValue, context: &mut Context) -> JsResult<serde_json::Value> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(serde_json::Value::Null))
        .map_err(|error| {
            JsNativeError::error()
                .with_message(error.to_string())
                .into()
        })
}
