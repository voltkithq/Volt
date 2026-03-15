use boa_engine::{Context, JsValue};
use volt_core::{fs, watcher};

use crate::modules::{promise_from_json_result, promise_from_result};

use super::shared::{base_dir, require_fs_permission, scoped_base_dir};

pub(super) fn watch_start(
    path: String,
    recursive: bool,
    debounce_ms: f64,
    context: &mut Context,
) -> JsValue {
    let result = (|| {
        let base = base_dir()?;
        let target = base.join(&path);
        watcher::start_watch(target, recursive, debounce_ms as u64)
    })();

    promise_from_result(context, result).into()
}

pub(super) fn watch_poll(watcher_id: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_fs_permission()?;
        let events = watcher::drain_events(&watcher_id)?;
        let json_events: Vec<serde_json::Value> = events
            .into_iter()
            .map(|event| serde_json::to_value(event).unwrap_or(serde_json::Value::Null))
            .collect();
        Ok(serde_json::Value::Array(json_events))
    })();

    promise_from_json_result(context, result).into()
}

pub(super) fn watch_close(watcher_id: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_fs_permission()?;
        watcher::stop_watch(&watcher_id)
    })();

    promise_from_result(context, result).into()
}

pub(super) fn scoped_watch_start(
    grant_id: String,
    subpath: String,
    recursive: bool,
    debounce_ms: f64,
    context: &mut Context,
) -> JsValue {
    let result = (|| {
        let base = scoped_base_dir(&grant_id)?;
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

pub(super) fn scoped_watch_poll(watcher_id: String, context: &mut Context) -> JsValue {
    watch_poll(watcher_id, context)
}

pub(super) fn scoped_watch_close(watcher_id: String, context: &mut Context) -> JsValue {
    watch_close(watcher_id, context)
}
