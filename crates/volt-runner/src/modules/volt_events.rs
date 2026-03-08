use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use volt_core::command::{self, AppCommand};

use super::{js_error, native_function_module, value_to_json};

fn normalize_event_name(event_name: String) -> Result<String, String> {
    let trimmed = event_name.trim();
    if trimmed.is_empty() {
        return Err("event name must not be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_window_id(window_id: String) -> Result<String, String> {
    let trimmed = window_id.trim();
    if trimmed.is_empty() {
        return Err("window id must not be empty".to_string());
    }
    Ok(trimmed.to_string())
}

fn emit_event(
    js_window_id: Option<String>,
    event_name: String,
    data: serde_json::Value,
) -> Result<(), String> {
    command::send_command(AppCommand::EmitEvent {
        js_window_id,
        event_name: normalize_event_name(event_name)?,
        data,
    })
    .map_err(|error| format!("failed to emit event: {error}"))
}

fn emit(event_name: String, data: Option<JsValue>, context: &mut Context) -> JsResult<()> {
    let payload = data
        .map(|value| value_to_json(value, context))
        .transpose()
        .map_err(|error| {
            js_error(
                "volt:events",
                "emit",
                format!("invalid event payload: {error}"),
            )
        })?;
    emit_event(None, event_name, payload.unwrap_or(serde_json::Value::Null))
        .map_err(|error| js_error("volt:events", "emit", error))
}

fn emit_to(
    window_id: String,
    event_name: String,
    data: Option<JsValue>,
    context: &mut Context,
) -> JsResult<()> {
    let payload = data
        .map(|value| value_to_json(value, context))
        .transpose()
        .map_err(|error| {
            js_error(
                "volt:events",
                "emitTo",
                format!("invalid event payload: {error}"),
            )
        })?;
    let normalized_window_id =
        normalize_window_id(window_id).map_err(|error| js_error("volt:events", "emitTo", error))?;
    emit_event(
        Some(normalized_window_id),
        event_name,
        payload.unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| js_error("volt:events", "emitTo", error))
}

pub fn build_module(context: &mut Context) -> Module {
    let emit = emit.into_js_function_copied(context);
    let emit_to = emit_to.into_js_function_copied(context);

    native_function_module(context, vec![("emit", emit), ("emitTo", emit_to)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_utils::{init_test_bridge, shutdown_test_bridge, test_guard};
    use std::time::Duration;

    #[test]
    fn emit_event_dispatches_broadcast_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, _proxy) = init_test_bridge();

        emit_event(
            None,
            "app:ready".to_string(),
            serde_json::json!({"ok": true}),
        )
        .expect("emit event");
        let envelope = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("command envelope");

        match envelope.command {
            AppCommand::EmitEvent {
                js_window_id,
                event_name,
                data,
            } => {
                assert!(js_window_id.is_none());
                assert_eq!(event_name, "app:ready");
                assert_eq!(data, serde_json::json!({"ok": true}));
            }
            command => panic!("unexpected command: {command:?}"),
        }

        shutdown_test_bridge(lifecycle);
    }

    #[test]
    fn emit_event_dispatches_targeted_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, _proxy) = init_test_bridge();

        emit_event(
            Some("window-3".to_string()),
            "progress".to_string(),
            serde_json::json!({"percent": 50}),
        )
        .expect("emit event");
        let envelope = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("command envelope");

        match envelope.command {
            AppCommand::EmitEvent {
                js_window_id,
                event_name,
                data,
            } => {
                assert_eq!(js_window_id.as_deref(), Some("window-3"));
                assert_eq!(event_name, "progress");
                assert_eq!(data, serde_json::json!({"percent": 50}));
            }
            command => panic!("unexpected command: {command:?}"),
        }

        shutdown_test_bridge(lifecycle);
    }

    #[test]
    fn normalize_event_name_rejects_empty_values() {
        assert!(normalize_event_name("".to_string()).is_err());
        assert!(normalize_event_name("   ".to_string()).is_err());
    }
}
