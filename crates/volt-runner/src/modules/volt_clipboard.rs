use boa_engine::{Context, IntoJsFunctionCopied, JsResult, Module};
use volt_core::clipboard;
use volt_core::permissions::Permission;

use super::{js_error, native_function_module, require_permission};

fn read_text() -> JsResult<String> {
    require_permission(Permission::Clipboard)?;
    clipboard::read_text().map_err(|error| {
        js_error(
            "volt:clipboard",
            "readText",
            format!("clipboard read failed: {error}"),
        )
    })
}

fn write_text(text: String) -> JsResult<()> {
    require_permission(Permission::Clipboard)?;
    clipboard::write_text(&text).map_err(|error| {
        js_error(
            "volt:clipboard",
            "writeText",
            format!("clipboard write failed: {error}"),
        )
    })
}

pub fn build_module(context: &mut Context) -> Module {
    let read_text = read_text.into_js_function_copied(context);
    let write_text = write_text.into_js_function_copied(context);

    native_function_module(
        context,
        vec![("readText", read_text), ("writeText", write_text)],
    )
}
