use boa_engine::{Context, JsValue};
use volt_core::fs;

use crate::modules::{promise_from_json_result, promise_from_result};

use super::shared::{stat_json, with_scoped_base_dir};

pub(super) fn scoped_read_file(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::read_file_text(base, &path).map_err(|error| format!("fs read failed: {error}"))
    });
    promise_from_json_result(context, result.map(serde_json::Value::String)).into()
}

pub(super) fn scoped_read_dir(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::read_dir(base, &path).map_err(|error| format!("fs read dir failed: {error}"))
    });
    promise_from_json_result(context, result.map(|entries| serde_json::json!(entries))).into()
}

pub(super) fn scoped_stat(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::stat(base, &path)
            .map(stat_json)
            .map_err(|error| format!("fs stat failed: {error}"))
    });
    promise_from_json_result(context, result).into()
}

pub(super) fn scoped_exists(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::exists(base, &path).map_err(|error| format!("fs exists failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_write_file(
    grant_id: String,
    path: String,
    data: String,
    context: &mut Context,
) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::write_file(base, &path, data.as_bytes())
            .map_err(|error| format!("fs write failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_mkdir(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::mkdir(base, &path).map_err(|error| format!("fs mkdir failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_remove(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::remove(base, &path).map_err(|error| format!("fs remove failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_rename(
    grant_id: String,
    from: String,
    to: String,
    context: &mut Context,
) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::rename(base, &from, &to).map_err(|error| format!("fs rename failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_copy(
    grant_id: String,
    from: String,
    to: String,
    context: &mut Context,
) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::copy(base, &from, &to).map_err(|error| format!("fs copy failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn scoped_read_file_binary(
    grant_id: String,
    path: String,
    context: &mut Context,
) -> JsValue {
    let result = with_scoped_base_dir(&grant_id, |base| {
        fs::read_file(base, &path)
            .map(|data| serde_json::json!(data))
            .map_err(|error| format!("fs read failed: {error}"))
    });
    promise_from_json_result(context, result).into()
}
