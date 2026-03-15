use boa_engine::{Context, JsResult, JsValue};

use super::bridge_support::{request_value, request_void, required_string};

pub(super) fn fs_read_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:read-file", args, context)
}

pub(super) fn fs_write_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = required_string(args, 0, context, "path")?;
    let data = required_string(args, 1, context, "file contents")?;
    request_void(
        "plugin:fs:write-file",
        serde_json::json!({ "path": path, "data": data }),
    )
}

pub(super) fn fs_read_dir(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:read-dir", args, context)
}

pub(super) fn fs_stat(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:stat", args, context)
}

pub(super) fn fs_exists(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:exists", args, context)
}

pub(super) fn fs_mkdir(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:mkdir", args, context)
}

pub(super) fn fs_remove(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    path_request_value("plugin:fs:remove", args, context)
}

pub(super) fn grant_fs_read_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:read-file", args, context)
}

pub(super) fn grant_fs_write_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let grant_id = required_string(args, 0, context, "grant id")?;
    let path = required_string(args, 1, context, "path")?;
    let data = required_string(args, 2, context, "file contents")?;
    request_void(
        "plugin:grant-fs:write-file",
        serde_json::json!({ "grantId": grant_id, "path": path, "data": data }),
    )
}

pub(super) fn grant_fs_read_dir(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:read-dir", args, context)
}

pub(super) fn grant_fs_stat(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:stat", args, context)
}

pub(super) fn grant_fs_exists(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:exists", args, context)
}

pub(super) fn grant_fs_mkdir(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:mkdir", args, context)
}

pub(super) fn grant_fs_remove(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    grant_request_value("plugin:grant-fs:remove", args, context)
}

fn path_request_value(method: &str, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = required_string(args, 0, context, "path")?;
    request_value(method, serde_json::json!({ "path": path }), context)
}

fn grant_request_value(method: &str, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let grant_id = required_string(args, 0, context, "grant id")?;
    let path = required_string(args, 1, context, "path")?;
    request_value(
        method,
        serde_json::json!({ "grantId": grant_id, "path": path }),
        context,
    )
}
