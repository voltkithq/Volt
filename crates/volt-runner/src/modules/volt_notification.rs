use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use volt_core::notification::{self, NotificationConfig};
use volt_core::permissions::Permission;

use super::{js_error, native_function_module, require_permission, value_to_json};

fn show(options: JsValue, context: &mut Context) -> JsResult<()> {
    require_permission(Permission::Notification)?;
    let options = value_to_json(options, context)
        .map_err(|error| js_error("volt:notification", "show", error))?;

    let title = options
        .get("title")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            js_error(
                "volt:notification",
                "show",
                "notification requires a 'title' field",
            )
        })?
        .to_string();

    let body = options
        .get("body")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    let icon = options
        .get("icon")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    let config = NotificationConfig { title, body, icon };
    notification::show_notification(&config).map_err(|error| {
        js_error(
            "volt:notification",
            "show",
            format!("notification failed: {error}"),
        )
    })
}

pub fn build_module(context: &mut Context) -> Module {
    let show = show.into_js_function_copied(context);
    native_function_module(context, vec![("show", show)])
}
