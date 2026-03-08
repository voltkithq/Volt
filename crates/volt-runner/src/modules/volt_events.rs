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
    use std::sync::mpsc;
    use std::sync::{Mutex, OnceLock};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use tao::event::Event;
    use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
    use volt_core::app::AppEvent;
    use volt_core::command::CommandEnvelope;

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn spawn_live_event_loop() -> (EventLoopProxy<AppEvent>, JoinHandle<()>) {
        let (proxy_tx, proxy_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut builder = EventLoopBuilder::<AppEvent>::with_user_event();
            #[cfg(target_os = "windows")]
            {
                use tao::platform::windows::EventLoopBuilderExtWindows;
                builder.with_any_thread(true);
            }
            #[cfg(any(
                target_os = "linux",
                target_os = "dragonfly",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd"
            ))]
            {
                use tao::platform::unix::EventLoopBuilderExtUnix;
                builder.with_any_thread(true);
            }

            let event_loop = builder.build();
            let proxy = event_loop.create_proxy();
            let _ = proxy_tx.send(proxy);
            event_loop.run(move |event, _, control_flow| {
                *control_flow = ControlFlow::Wait;
                if let Event::UserEvent(AppEvent::Quit) = event {
                    *control_flow = ControlFlow::Exit;
                }
            });
        });

        let proxy = proxy_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("event loop proxy");
        (proxy, handle)
    }

    fn init_test_bridge() -> (
        mpsc::Receiver<CommandEnvelope>,
        command::BridgeLifecycle,
        EventLoopProxy<AppEvent>,
        JoinHandle<()>,
    ) {
        command::shutdown_bridge();
        let (proxy, handle) = spawn_live_event_loop();
        let registration = command::init_bridge(proxy.clone()).expect("bridge init");
        (registration.receiver, registration.lifecycle, proxy, handle)
    }

    fn shutdown_test_bridge(
        lifecycle: command::BridgeLifecycle,
        proxy: EventLoopProxy<AppEvent>,
        handle: JoinHandle<()>,
    ) {
        lifecycle.shutdown();
        let _ = proxy.send_event(AppEvent::Quit);
        let _ = handle.join();
    }

    #[test]
    fn emit_event_dispatches_broadcast_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

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

        shutdown_test_bridge(lifecycle, proxy, handle);
    }

    #[test]
    fn emit_event_dispatches_targeted_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

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

        shutdown_test_bridge(lifecycle, proxy, handle);
    }

    #[test]
    fn normalize_event_name_rejects_empty_values() {
        assert!(normalize_event_name("".to_string()).is_err());
        assert!(normalize_event_name("   ".to_string()).is_err());
    }
}
