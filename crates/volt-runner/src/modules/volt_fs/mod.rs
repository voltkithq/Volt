mod base_ops;
mod scope_binding;
mod scoped_ops;
mod shared;
mod watchers;

use boa_engine::{Context, IntoJsFunctionCopied, Module};

use crate::modules::native_function_module;

pub fn build_module(context: &mut Context) -> Module {
    let read_file = base_ops::read_file.into_js_function_copied(context);
    let write_file = base_ops::write_file.into_js_function_copied(context);
    let read_dir = base_ops::read_dir.into_js_function_copied(context);
    let exists = base_ops::exists.into_js_function_copied(context);
    let stat = base_ops::stat.into_js_function_copied(context);
    let mkdir = base_ops::mkdir.into_js_function_copied(context);
    let remove = base_ops::remove.into_js_function_copied(context);
    let bind_scope = scope_binding::bind_scope.into_js_function_copied(context);
    let scoped_read_file = scoped_ops::scoped_read_file.into_js_function_copied(context);
    let scoped_read_dir = scoped_ops::scoped_read_dir.into_js_function_copied(context);
    let scoped_stat = scoped_ops::scoped_stat.into_js_function_copied(context);
    let scoped_exists = scoped_ops::scoped_exists.into_js_function_copied(context);
    let scoped_read_file_binary =
        scoped_ops::scoped_read_file_binary.into_js_function_copied(context);
    let scoped_write_file = scoped_ops::scoped_write_file.into_js_function_copied(context);
    let scoped_mkdir = scoped_ops::scoped_mkdir.into_js_function_copied(context);
    let scoped_remove = scoped_ops::scoped_remove.into_js_function_copied(context);
    let scoped_rename = scoped_ops::scoped_rename.into_js_function_copied(context);
    let scoped_copy = scoped_ops::scoped_copy.into_js_function_copied(context);
    let watch_start = watchers::watch_start.into_js_function_copied(context);
    let watch_poll = watchers::watch_poll.into_js_function_copied(context);
    let watch_close = watchers::watch_close.into_js_function_copied(context);
    let scoped_watch_start = watchers::scoped_watch_start.into_js_function_copied(context);
    let scoped_watch_poll = watchers::scoped_watch_poll.into_js_function_copied(context);
    let scoped_watch_close = watchers::scoped_watch_close.into_js_function_copied(context);

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
