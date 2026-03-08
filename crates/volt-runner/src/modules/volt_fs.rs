use boa_engine::{Context, IntoJsFunctionCopied, JsValue, Module};
use volt_core::fs;
use volt_core::permissions::Permission;

use super::{
    fs_base_dir, native_function_module, promise_from_json_result, promise_from_result,
    require_permission,
};

fn read_file(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        fs::read_file_text(&base, &path).map_err(|error| format!("fs read failed: {error}"))
    })();

    promise_from_json_result(context, result.map(serde_json::Value::String)).into()
}

fn write_file(path: String, data: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        fs::write_file(&base, &path, data.as_bytes())
            .map_err(|error| format!("fs write failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn read_dir(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        fs::read_dir(&base, &path).map_err(|error| format!("fs read dir failed: {error}"))
    })();

    promise_from_json_result(context, result.map(|entries| serde_json::json!(entries))).into()
}

fn exists(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        let resolved =
            fs::safe_resolve(&base, &path).map_err(|error| format!("fs exists failed: {error}"))?;
        Ok(resolved.exists())
    })();

    promise_from_result(context, result).into()
}

fn mkdir(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        fs::mkdir(&base, &path).map_err(|error| format!("fs mkdir failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn remove(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        fs::remove(&base, &path).map_err(|error| format!("fs remove failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

pub fn build_module(context: &mut Context) -> Module {
    let read_file = read_file.into_js_function_copied(context);
    let write_file = write_file.into_js_function_copied(context);
    let read_dir = read_dir.into_js_function_copied(context);
    let exists = exists.into_js_function_copied(context);
    let mkdir = mkdir.into_js_function_copied(context);
    let remove = remove.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("readFile", read_file),
            ("writeFile", write_file),
            ("readDir", read_dir),
            ("exists", exists),
            ("mkdir", mkdir),
            ("remove", remove),
        ],
    )
}
