use std::rc::Rc;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, IntoJsFunctionCopied, JsArgs, JsResult, JsValue, Module, js_string};
use volt_core::fs;
use volt_core::grant_store;
use volt_core::permissions::Permission;

use volt_core::watcher;

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
        fs::exists(&base, &path).map_err(|error| format!("fs exists failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn stat(path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        let info = fs::stat(&base, &path).map_err(|error| format!("fs stat failed: {error}"))?;
        Ok(serde_json::json!({
            "size": info.size,
            "isFile": info.is_file,
            "isDir": info.is_dir,
            "readonly": info.readonly,
            "modifiedMs": info.modified_ms,
            "createdMs": info.created_ms,
        }))
    })();

    promise_from_json_result(context, result).into()
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

/// Helper to extract argument N as a String from JsValue args.
fn arg_string(args: &[JsValue], index: usize, context: &mut Context) -> JsResult<String> {
    args.get_or_undefined(index)
        .to_string(context)
        .map(|s| s.to_std_string_escaped())
}

fn bind_scope(grant_id: String, context: &mut Context) -> JsValue {
    let result = (|| -> Result<JsValue, String> {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;

        let gid: Rc<str> = Rc::from(grant_id.as_str());

        // Each method closure captures the grant ID via Rc<str> (which is Copy-friendly via clone)
        // SAFETY: These closures capture Rc<str> which is not Copy but is safe to use
        // in Boa's single-threaded JS context. The closures do not outlive the context.
        let make_method = |gid: Rc<str>, f: fn(String, String, &mut Context) -> JsValue| {
            let gid = gid.clone();
            unsafe {
                NativeFunction::from_closure(move |_this, args, ctx| {
                    let path = arg_string(args, 0, ctx)?;
                    Ok(f(gid.to_string(), path, ctx))
                })
            }
        };

        let make_method2 =
            |gid: Rc<str>, f: fn(String, String, String, &mut Context) -> JsValue| {
                let gid = gid.clone();
                unsafe {
                    NativeFunction::from_closure(move |_this, args, ctx| {
                        let a = arg_string(args, 0, ctx)?;
                        let b = arg_string(args, 1, ctx)?;
                        Ok(f(gid.to_string(), a, b, ctx))
                    })
                }
            };

        let watch_gid = gid.clone();
        let watch_fn = unsafe {
            NativeFunction::from_closure(move |_this, args, ctx| {
                let subpath = arg_string(args, 0, ctx)?;
                let recursive = args.get_or_undefined(1).to_boolean();
                let debounce = args.get_or_undefined(2).to_number(ctx).unwrap_or(200.0);
                Ok(scoped_watch_start(
                    watch_gid.to_string(),
                    subpath,
                    recursive,
                    debounce,
                    ctx,
                ))
            })
        };

        let obj = ObjectInitializer::new(context)
            .function(
                make_method(gid.clone(), scoped_read_file),
                js_string!("readFile"),
                1,
            )
            .function(
                make_method(gid.clone(), scoped_read_file_binary),
                js_string!("readFileBinary"),
                1,
            )
            .function(
                make_method(gid.clone(), scoped_read_dir),
                js_string!("readDir"),
                1,
            )
            .function(make_method(gid.clone(), scoped_stat), js_string!("stat"), 1)
            .function(
                make_method(gid.clone(), scoped_exists),
                js_string!("exists"),
                1,
            )
            .function(
                make_method2(gid.clone(), scoped_write_file),
                js_string!("writeFile"),
                2,
            )
            .function(
                make_method(gid.clone(), scoped_mkdir),
                js_string!("mkdir"),
                1,
            )
            .function(
                make_method(gid.clone(), scoped_remove),
                js_string!("remove"),
                1,
            )
            .function(
                make_method2(gid.clone(), scoped_rename),
                js_string!("rename"),
                2,
            )
            .function(
                make_method2(gid.clone(), scoped_copy),
                js_string!("copy"),
                2,
            )
            .function(watch_fn, js_string!("watch"), 3)
            .property(
                js_string!("grantId"),
                JsValue::from(js_string!(grant_id.as_str())),
                Attribute::READONLY,
            )
            .build();

        Ok(obj.into())
    })();

    match result {
        Ok(obj) => super::resolve_promise(context, obj).into(),
        Err(msg) => super::reject_promise(context, msg).into(),
    }
}

fn scoped_read_file(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::read_file_text(&base, &path).map_err(|error| format!("fs read failed: {error}"))
    })();

    promise_from_json_result(context, result.map(serde_json::Value::String)).into()
}

fn scoped_read_dir(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::read_dir(&base, &path).map_err(|error| format!("fs read dir failed: {error}"))
    })();

    promise_from_json_result(context, result.map(|entries| serde_json::json!(entries))).into()
}

fn scoped_stat(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        let info = fs::stat(&base, &path).map_err(|error| format!("fs stat failed: {error}"))?;
        Ok(serde_json::json!({
            "size": info.size,
            "isFile": info.is_file,
            "isDir": info.is_dir,
            "readonly": info.readonly,
            "modifiedMs": info.modified_ms,
            "createdMs": info.created_ms,
        }))
    })();

    promise_from_json_result(context, result).into()
}

fn scoped_exists(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::exists(&base, &path).map_err(|error| format!("fs exists failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_write_file(
    grant_id: String,
    path: String,
    data: String,
    context: &mut Context,
) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::write_file(&base, &path, data.as_bytes())
            .map_err(|error| format!("fs write failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_mkdir(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::mkdir(&base, &path).map_err(|error| format!("fs mkdir failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_remove(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::remove(&base, &path).map_err(|error| format!("fs remove failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_rename(grant_id: String, from: String, to: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::rename(&base, &from, &to).map_err(|error| format!("fs rename failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_copy(grant_id: String, from: String, to: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        fs::copy(&base, &from, &to).map_err(|error| format!("fs copy failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn scoped_read_file_binary(grant_id: String, path: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        let data =
            fs::read_file(&base, &path).map_err(|error| format!("fs read failed: {error}"))?;
        Ok(serde_json::json!(data))
    })();

    promise_from_json_result(context, result).into()
}

fn watch_start(path: String, recursive: bool, debounce_ms: f64, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = fs_base_dir()?;
        let target = base.join(&path);
        watcher::start_watch(target, recursive, debounce_ms as u64)
    })();

    promise_from_result(context, result).into()
}

fn watch_poll(watcher_id: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let events = watcher::drain_events(&watcher_id)?;
        let json_events: Vec<serde_json::Value> = events
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();
        Ok(serde_json::Value::Array(json_events))
    })();

    promise_from_json_result(context, result).into()
}

fn watch_close(watcher_id: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        watcher::stop_watch(&watcher_id)
    })();

    promise_from_result(context, result).into()
}

fn scoped_watch_start(
    grant_id: String,
    subpath: String,
    recursive: bool,
    debounce_ms: f64,
    context: &mut Context,
) -> JsValue {
    let result = (|| {
        require_permission(Permission::FileSystem).map_err(super::format_js_error)?;
        let base = grant_store::resolve_grant(&grant_id).map_err(|error| format!("{error}"))?;
        let target = if subpath.is_empty() {
            base
        } else {
            fs::safe_resolve(&base, &subpath)
                .map_err(|error| format!("watch path invalid: {error}"))?
        };
        watcher::start_watch(target, recursive, debounce_ms as u64)
    })();

    promise_from_result(context, result).into()
}

fn scoped_watch_poll(watcher_id: String, context: &mut Context) -> JsValue {
    watch_poll(watcher_id, context)
}

fn scoped_watch_close(watcher_id: String, context: &mut Context) -> JsValue {
    watch_close(watcher_id, context)
}

pub fn build_module(context: &mut Context) -> Module {
    let read_file = read_file.into_js_function_copied(context);
    let write_file = write_file.into_js_function_copied(context);
    let read_dir = read_dir.into_js_function_copied(context);
    let exists = exists.into_js_function_copied(context);
    let stat = stat.into_js_function_copied(context);
    let mkdir = mkdir.into_js_function_copied(context);
    let remove = remove.into_js_function_copied(context);
    let bind_scope = bind_scope.into_js_function_copied(context);
    let scoped_read_file = scoped_read_file.into_js_function_copied(context);
    let scoped_read_dir = scoped_read_dir.into_js_function_copied(context);
    let scoped_stat = scoped_stat.into_js_function_copied(context);
    let scoped_exists = scoped_exists.into_js_function_copied(context);
    let scoped_read_file_binary = scoped_read_file_binary.into_js_function_copied(context);
    let scoped_write_file = scoped_write_file.into_js_function_copied(context);
    let scoped_mkdir = scoped_mkdir.into_js_function_copied(context);
    let scoped_remove = scoped_remove.into_js_function_copied(context);
    let scoped_rename = scoped_rename.into_js_function_copied(context);
    let scoped_copy = scoped_copy.into_js_function_copied(context);
    let watch_start = watch_start.into_js_function_copied(context);
    let watch_poll = watch_poll.into_js_function_copied(context);
    let watch_close = watch_close.into_js_function_copied(context);
    let scoped_watch_start = scoped_watch_start.into_js_function_copied(context);
    let scoped_watch_poll = scoped_watch_poll.into_js_function_copied(context);
    let scoped_watch_close = scoped_watch_close.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("readFile", read_file),
            ("writeFile", write_file),
            ("readDir", read_dir),
            ("exists", exists),
            ("stat", stat),
            ("mkdir", mkdir),
            ("remove", remove),
            ("bindScope", bind_scope),
            ("scopedReadFile", scoped_read_file),
            ("scopedReadDir", scoped_read_dir),
            ("scopedStat", scoped_stat),
            ("scopedExists", scoped_exists),
            ("scopedReadFileBinary", scoped_read_file_binary),
            ("scopedWriteFile", scoped_write_file),
            ("scopedMkdir", scoped_mkdir),
            ("scopedRemove", scoped_remove),
            ("scopedRename", scoped_rename),
            ("scopedCopy", scoped_copy),
            ("watchStart", watch_start),
            ("watchPoll", watch_poll),
            ("watchClose", watch_close),
            ("scopedWatchStart", scoped_watch_start),
            ("scopedWatchPoll", scoped_watch_poll),
            ("scopedWatchClose", scoped_watch_close),
        ],
    )
}
