use boa_engine::{Context, JsValue};
use volt_core::fs;

use crate::modules::{promise_from_json_result, promise_from_result};

use super::shared::{stat_json, with_base_dir};

pub(super) fn read_file(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::read_file_text(base, &path).map_err(|error| format!("fs read failed: {error}"))
    });
    promise_from_json_result(context, result.map(serde_json::Value::String)).into()
}

pub(super) fn write_file(path: String, data: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::write_file(base, &path, data.as_bytes())
            .map_err(|error| format!("fs write failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn read_dir(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::read_dir(base, &path).map_err(|error| format!("fs read dir failed: {error}"))
    });
    promise_from_json_result(context, result.map(|entries| serde_json::json!(entries))).into()
}

pub(super) fn exists(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::exists(base, &path).map_err(|error| format!("fs exists failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn stat(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::stat(base, &path)
            .map(stat_json)
            .map_err(|error| format!("fs stat failed: {error}"))
    });
    promise_from_json_result(context, result).into()
}

pub(super) fn mkdir(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::mkdir(base, &path).map_err(|error| format!("fs mkdir failed: {error}"))
    });
    promise_from_result(context, result).into()
}

pub(super) fn remove(path: String, context: &mut Context) -> JsValue {
    let result = with_base_dir(|base| {
        fs::remove(base, &path).map_err(|error| format!("fs remove failed: {error}"))
    });
    promise_from_result(context, result).into()
}
