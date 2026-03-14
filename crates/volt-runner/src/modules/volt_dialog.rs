use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use volt_core::dialog;
use volt_core::grant_store;
use volt_core::permissions::Permission;

use super::{
    dialog_async, native_function_module, reject_promise, require_permission, value_to_json,
};

fn show_open(options: Option<JsValue>, context: &mut Context) -> JsValue {
    if let Err(e) = require_permission(Permission::Dialog) {
        return reject_promise(context, super::format_js_error(e)).into();
    }

    let parsed = match parse_open_options(options, context) {
        Ok(opts) => opts,
        Err(message) => return reject_promise(context, message).into(),
    };

    dialog_async::spawn_dialog(context, move || {
        let selected = dialog::show_open_dialog(&parsed)
            .into_iter()
            .next()
            .map(|path| path.to_string_lossy().into_owned());
        Ok(serde_json::json!(selected))
    })
    .into()
}

fn show_save(options: Option<JsValue>, context: &mut Context) -> JsValue {
    if let Err(e) = require_permission(Permission::Dialog) {
        return reject_promise(context, super::format_js_error(e)).into();
    }

    let parsed = match parse_save_options(options, context) {
        Ok(opts) => opts,
        Err(message) => return reject_promise(context, message).into(),
    };

    dialog_async::spawn_dialog(context, move || {
        let selected =
            dialog::show_save_dialog(&parsed).map(|path| path.to_string_lossy().into_owned());
        Ok(serde_json::json!(selected))
    })
    .into()
}

fn show_message(options: JsValue, context: &mut Context) -> JsValue {
    if let Err(e) = require_permission(Permission::Dialog) {
        return reject_promise(context, super::format_js_error(e)).into();
    }

    let parsed: dialog::MessageDialogOptions =
        match value_to_json(options, context).and_then(|json| {
            serde_json::from_value(json)
                .map_err(|error| format!("invalid message dialog options: {error}"))
        }) {
            Ok(opts) => opts,
            Err(message) => return reject_promise(context, message).into(),
        };

    dialog_async::spawn_dialog(context, move || {
        let accepted = dialog::show_message_dialog(&parsed);
        Ok(serde_json::json!(if accepted { 1 } else { 0 }))
    })
    .into()
}

fn show_open_with_grant(options: Option<JsValue>, context: &mut Context) -> JsValue {
    if let Err(e) = require_permission(Permission::Dialog) {
        return reject_promise(context, super::format_js_error(e)).into();
    }
    if let Err(e) = require_permission(Permission::FileSystem) {
        return reject_promise(context, super::format_js_error(e)).into();
    }

    let mut parsed = match parse_open_options(options, context) {
        Ok(opts) => opts,
        Err(message) => return reject_promise(context, message).into(),
    };
    parsed.directory = true;

    dialog_async::spawn_dialog(context, move || {
        let selected = dialog::show_open_dialog(&parsed);
        let mut paths = Vec::new();
        let mut grant_ids = Vec::new();

        for path in selected {
            let path_str = path.to_string_lossy().into_owned();
            let grant_id = grant_store::create_grant(path)
                .map_err(|error| format!("failed to create grant: {error}"))?;
            paths.push(serde_json::Value::String(path_str));
            grant_ids.push(serde_json::Value::String(grant_id));
        }

        Ok(serde_json::json!({
            "paths": paths,
            "grantIds": grant_ids,
        }))
    })
    .into()
}

fn parse_open_options(
    options: Option<JsValue>,
    context: &mut Context,
) -> Result<dialog::OpenDialogOptions, String> {
    let json = options
        .map(|value| value_to_json(value, context))
        .transpose()?
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    serde_json::from_value(json).map_err(|error| format!("invalid open dialog options: {error}"))
}

fn parse_save_options(
    options: Option<JsValue>,
    context: &mut Context,
) -> Result<dialog::SaveDialogOptions, String> {
    let json = options
        .map(|value| value_to_json(value, context))
        .transpose()?
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    serde_json::from_value(json).map_err(|error| format!("invalid save dialog options: {error}"))
}

pub fn build_module(context: &mut Context) -> Module {
    let show_open = show_open.into_js_function_copied(context);
    let show_save = show_save.into_js_function_copied(context);
    let show_message = show_message.into_js_function_copied(context);
    let show_open_with_grant = show_open_with_grant.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("showOpen", show_open),
            ("showSave", show_save),
            ("showMessage", show_message),
            ("showOpenWithGrant", show_open_with_grant),
        ],
    )
}
