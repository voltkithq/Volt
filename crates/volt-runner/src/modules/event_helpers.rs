use boa_engine::object::builtins::JsFunction;
use boa_engine::{Context, JsResult};
use volt_core::permissions::Permission;

use super::{bind_native_event_handler, js_error, require_permission_message};

const NATIVE_EVENT_ON_GLOBAL: &str = "__volt_native_event_on__";
const NATIVE_EVENT_OFF_GLOBAL: &str = "__volt_native_event_off__";

fn bind_native_event_listener<F>(
    context: &mut Context,
    module_name: &'static str,
    permission: Permission,
    is_on: bool,
    event_name: String,
    handler: JsFunction,
    normalize_event_name: F,
) -> JsResult<()>
where
    F: FnOnce(String) -> Result<&'static str, String>,
{
    let api_function = if is_on { "on" } else { "off" };
    let global_name = if is_on {
        NATIVE_EVENT_ON_GLOBAL
    } else {
        NATIVE_EVENT_OFF_GLOBAL
    };
    require_permission_message(permission)
        .map_err(|error| js_error(module_name, api_function, error))?;
    let event_type = normalize_event_name(event_name)
        .map_err(|error| js_error(module_name, api_function, error))?;
    bind_native_event_handler(
        context,
        module_name,
        api_function,
        global_name,
        event_type,
        handler,
    )
}

pub(crate) fn bind_native_event_on<F>(
    context: &mut Context,
    module_name: &'static str,
    permission: Permission,
    event_name: String,
    handler: JsFunction,
    normalize_event_name: F,
) -> JsResult<()>
where
    F: FnOnce(String) -> Result<&'static str, String>,
{
    bind_native_event_listener(
        context,
        module_name,
        permission,
        true,
        event_name,
        handler,
        normalize_event_name,
    )
}

pub(crate) fn bind_native_event_off<F>(
    context: &mut Context,
    module_name: &'static str,
    permission: Permission,
    event_name: String,
    handler: JsFunction,
    normalize_event_name: F,
) -> JsResult<()>
where
    F: FnOnce(String) -> Result<&'static str, String>,
{
    bind_native_event_listener(
        context,
        module_name,
        permission,
        false,
        event_name,
        handler,
        normalize_event_name,
    )
}
