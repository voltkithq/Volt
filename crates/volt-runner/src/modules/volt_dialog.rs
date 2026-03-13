use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use volt_core::dialog;
use volt_core::grant_store;
use volt_core::permissions::Permission;

use super::{
    native_function_module, promise_from_json_result, promise_from_result, require_permission,
    value_to_json,
};

fn show_open(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Dialog).map_err(super::format_js_error)?;
        let options = options
            .map(|value| value_to_json(value, context))
            .transpose()?
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let parsed: dialog::OpenDialogOptions = serde_json::from_value(options)
            .map_err(|error| format!("invalid open dialog options: {error}"))?;

        let selected = dialog::show_open_dialog(&parsed)
            .into_iter()
            .next()
            .map(|path| path.to_string_lossy().into_owned());
        Ok(selected)
    })();

    promise_from_json_result(context, result.map(|selected| serde_json::json!(selected))).into()
}

fn show_save(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Dialog).map_err(super::format_js_error)?;
        let options = options
            .map(|value| value_to_json(value, context))
            .transpose()?
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let parsed: dialog::SaveDialogOptions = serde_json::from_value(options)
            .map_err(|error| format!("invalid save dialog options: {error}"))?;

        Ok(dialog::show_save_dialog(&parsed).map(|path| path.to_string_lossy().into_owned()))
    })();

    promise_from_json_result(context, result.map(|selected| serde_json::json!(selected))).into()
}

fn show_message(options: JsValue, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Dialog).map_err(super::format_js_error)?;
        let options = value_to_json(options, context)?;
        let parsed: dialog::MessageDialogOptions = serde_json::from_value(options)
            .map_err(|error| format!("invalid message dialog options: {error}"))?;
        let accepted = dialog::show_message_dialog(&parsed);
        Ok(if accepted { 1 } else { 0 })
    })();

    promise_from_result(context, result).into()
}

fn show_open_with_grant(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Dialog).map_err(super::format_js_error)?;
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;

        let options = options
            .map(|value| value_to_json(value, context))
            .transpose()?
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let mut parsed: dialog::OpenDialogOptions = serde_json::from_value(options)
            .map_err(|error| format!("invalid open dialog options: {error}"))?;

        // Force directory mode for grant dialogs
        parsed.directory = true;

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
    })();

    promise_from_json_result(context, result).into()
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
