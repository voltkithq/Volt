use std::rc::Rc;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::ObjectInitializer;
use boa_engine::property::Attribute;
use boa_engine::{Context, JsValue, js_string};

use super::scoped_ops;
use super::shared::{arg_string, require_fs_permission};
use super::watchers;

pub(super) fn bind_scope(grant_id: String, context: &mut Context) -> JsValue {
    let result = (|| -> Result<JsValue, String> {
        require_fs_permission()?;
        volt_core::grant_store::resolve_grant(&grant_id).map_err(|error| error.to_string())?;

        let gid: Rc<str> = Rc::from(grant_id.as_str());
        let obj = ObjectInitializer::new(context)
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_read_file),
                js_string!("readFile"),
                1,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_read_file_binary),
                js_string!("readFileBinary"),
                1,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_read_dir),
                js_string!("readDir"),
                1,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_stat),
                js_string!("stat"),
                1,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_exists),
                js_string!("exists"),
                1,
            )
            .function(
                make_scoped_method2(gid.clone(), scoped_ops::scoped_write_file),
                js_string!("writeFile"),
                2,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_mkdir),
                js_string!("mkdir"),
                1,
            )
            .function(
                make_scoped_method(gid.clone(), scoped_ops::scoped_remove),
                js_string!("remove"),
                1,
            )
            .function(
                make_scoped_method2(gid.clone(), scoped_ops::scoped_rename),
                js_string!("rename"),
                2,
            )
            .function(
                make_scoped_method2(gid.clone(), scoped_ops::scoped_copy),
                js_string!("copy"),
                2,
            )
            .function(make_watch_method(gid.clone()), js_string!("watch"), 3)
            .property(
                js_string!("grantId"),
                JsValue::from(js_string!(grant_id.as_str())),
                Attribute::READONLY,
            )
            .build();

        Ok(obj.into())
    })();

    match result {
        Ok(obj) => crate::modules::resolve_promise(context, obj).into(),
        Err(message) => crate::modules::reject_promise(context, message).into(),
    }
}

fn make_scoped_method(
    grant_id: Rc<str>,
    function: fn(String, String, &mut Context) -> JsValue,
) -> NativeFunction {
    let grant_id = grant_id.clone();
    unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let path = arg_string(args, 0, ctx)?;
            Ok(function(grant_id.to_string(), path, ctx))
        })
    }
}

fn make_scoped_method2(
    grant_id: Rc<str>,
    function: fn(String, String, String, &mut Context) -> JsValue,
) -> NativeFunction {
    let grant_id = grant_id.clone();
    unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let a = arg_string(args, 0, ctx)?;
            let b = arg_string(args, 1, ctx)?;
            Ok(function(grant_id.to_string(), a, b, ctx))
        })
    }
}

fn make_watch_method(grant_id: Rc<str>) -> NativeFunction {
    unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let subpath = arg_string(args, 0, ctx)?;
            let recursive = args.get(1).is_some_and(JsValue::to_boolean);
            let debounce = args
                .get(2)
                .map(|value| value.to_number(ctx).unwrap_or(200.0))
                .unwrap_or(200.0);

            Ok(watchers::scoped_watch_start(
                grant_id.to_string(),
                subpath,
                recursive,
                debounce,
                ctx,
            ))
        })
    }
}
