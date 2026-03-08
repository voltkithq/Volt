use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use serde_json::Value;

use super::super::value_to_json;
use super::config::{
    check_for_update_with_public_key, embedded_update_public_key, ensure_update_version_is_newer,
    ensure_updater_permissions, parse_check_options_json, parse_update_info_json,
};
use super::events::{
    begin_update_install_operation, emit_update_ready_event, finish_update_install_operation,
    mark_active_update_install_cancelled,
};
use super::operations::download_and_install_with_public_key;
use super::serialization::{json_to_js_value, update_info_to_json};
use crate::modules::{
    native_function_module, promise_from_json_result, promise_from_result, reject_promise,
};

fn check_for_update(options: JsValue, context: &mut Context) -> JsValue {
    let result = (|| -> Result<Value, String> {
        ensure_updater_permissions()?;
        let options_json = value_to_json(options, context)
            .map_err(|error| format!("invalid update options: {error}"))?;
        let options = parse_check_options_json(options_json)?;
        let public_key = embedded_update_public_key()?;

        let update =
            std::thread::spawn(move || check_for_update_with_public_key(options, public_key))
                .join()
                .map_err(|_| "updater worker thread panicked".to_string())??;

        Ok(update.map(update_info_to_json).unwrap_or(Value::Null))
    })();

    match result {
        Ok(payload) => match json_to_js_value(context, &payload) {
            Ok(value) => promise_from_result(context, Ok(value)).into(),
            Err(error) => reject_promise(context, error).into(),
        },
        Err(error) => promise_from_json_result(context, Err(error)).into(),
    }
}

fn download_and_install(update_info: JsValue, context: &mut Context) -> JsValue {
    let result = (|| -> Result<(), String> {
        ensure_updater_permissions()?;
        let update_info_json = value_to_json(update_info, context)
            .map_err(|error| format!("invalid update info: {error}"))?;
        let info = parse_update_info_json(update_info_json)?;
        ensure_update_version_is_newer(&info.version)?;
        let event_version = info.version.clone();
        let public_key = embedded_update_public_key()?;
        let operation_id = begin_update_install_operation()?;

        let install_result = std::thread::spawn(move || {
            download_and_install_with_public_key(info, public_key, operation_id)
        })
        .join()
        .map_err(|_| "updater worker thread panicked".to_string())?;

        finish_update_install_operation(operation_id);
        install_result?;

        if let Err(error) = emit_update_ready_event(&event_version) {
            tracing::warn!(
                error = %error,
                version = %event_version,
                "failed to emit updater ready event"
            );
        }

        Ok(())
    })();

    promise_from_result(context, result).into()
}

fn cancel_download_and_install() -> JsResult<()> {
    ensure_updater_permissions().map_err(|error| {
        super::super::js_error("volt:updater", "cancelDownloadAndInstall", error)
    })?;
    mark_active_update_install_cancelled();
    Ok(())
}

pub fn build_module(context: &mut Context) -> Module {
    let check_for_update = check_for_update.into_js_function_copied(context);
    let download_and_install = download_and_install.into_js_function_copied(context);
    let cancel_download_and_install = cancel_download_and_install.into_js_function_copied(context);
    let exports = vec![
        ("checkForUpdate", check_for_update),
        ("downloadAndInstall", download_and_install),
        ("cancelDownloadAndInstall", cancel_download_and_install),
    ];
    native_function_module(context, exports)
}
