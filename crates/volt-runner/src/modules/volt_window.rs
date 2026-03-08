use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use volt_core::command::{self, AppCommand};

use super::{js_error, native_function_module, promise_from_result};

const DEFAULT_WINDOW_ID: &str = "window-1";

fn resolve_window_id(window_id: Option<String>) -> String {
    match window_id {
        Some(js_id) if !js_id.trim().is_empty() => js_id,
        _ => DEFAULT_WINDOW_ID.to_string(),
    }
}

fn send_window_command(
    window_id: Option<String>,
    build_command: impl FnOnce(String) -> AppCommand,
) -> Result<(), String> {
    let js_id = resolve_window_id(window_id);
    command::send_command(build_command(js_id))
        .map_err(|error| format!("failed to send window command: {error}"))
}

fn query_window_count() -> Result<u32, String> {
    command::send_query(|reply| AppCommand::GetWindowCount { reply })
        .map_err(|error| format!("failed to query window count: {error}"))
}

fn close(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::CloseWindow { js_id })
        .map_err(|error| js_error("volt:window", "close", error))
}

fn show(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::ShowWindow { js_id })
        .map_err(|error| js_error("volt:window", "show", error))
}

fn focus(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::FocusWindow { js_id })
        .map_err(|error| js_error("volt:window", "focus", error))
}

fn maximize(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::MaximizeWindow { js_id })
        .map_err(|error| js_error("volt:window", "maximize", error))
}

fn minimize(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::MinimizeWindow { js_id })
        .map_err(|error| js_error("volt:window", "minimize", error))
}

fn restore(window_id: Option<String>) -> JsResult<()> {
    send_window_command(window_id, |js_id| AppCommand::RestoreWindow { js_id })
        .map_err(|error| js_error("volt:window", "restore", error))
}

fn get_window_count(context: &mut Context) -> JsValue {
    promise_from_result(context, query_window_count()).into()
}

fn quit() -> JsResult<()> {
    command::send_command(AppCommand::Quit).map_err(|error| {
        js_error(
            "volt:window",
            "quit",
            format!("failed to quit app: {error}"),
        )
    })
}

pub fn build_module(context: &mut Context) -> Module {
    let close = close.into_js_function_copied(context);
    let show = show.into_js_function_copied(context);
    let focus = focus.into_js_function_copied(context);
    let maximize = maximize.into_js_function_copied(context);
    let minimize = minimize.into_js_function_copied(context);
    let restore = restore.into_js_function_copied(context);
    let get_window_count = get_window_count.into_js_function_copied(context);
    let quit = quit.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("close", close),
            ("show", show),
            ("focus", focus),
            ("maximize", maximize),
            ("minimize", minimize),
            ("restore", restore),
            ("getWindowCount", get_window_count),
            ("quit", quit),
        ],
    )
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
    fn resolve_window_id_defaults_to_first_window() {
        assert_eq!(resolve_window_id(None), DEFAULT_WINDOW_ID);
        assert_eq!(
            resolve_window_id(Some("   ".to_string())),
            DEFAULT_WINDOW_ID
        );
        assert_eq!(
            resolve_window_id(Some("window-8".to_string())),
            "window-8".to_string()
        );
    }

    #[test]
    fn close_uses_default_window_when_not_provided() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

        send_window_command(None, |js_id| AppCommand::CloseWindow { js_id })
            .expect("close command");
        let envelope = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("command envelope");

        match envelope.command {
            AppCommand::CloseWindow { js_id } => assert_eq!(js_id, DEFAULT_WINDOW_ID),
            command => panic!("unexpected command: {command:?}"),
        }

        shutdown_test_bridge(lifecycle, proxy, handle);
    }

    #[test]
    fn focus_forwards_explicit_window_id() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

        send_window_command(Some("window-99".to_string()), |js_id| {
            AppCommand::FocusWindow { js_id }
        })
        .expect("focus command");
        let envelope = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("command envelope");

        match envelope.command {
            AppCommand::FocusWindow { js_id } => assert_eq!(js_id, "window-99"),
            command => panic!("unexpected command: {command:?}"),
        }

        shutdown_test_bridge(lifecycle, proxy, handle);
    }

    #[test]
    fn query_window_count_uses_reply_channel() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

        let responder = std::thread::spawn(move || {
            let envelope = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("command envelope");
            match envelope.command {
                AppCommand::GetWindowCount { reply } => {
                    let _ = reply.send(3);
                }
                command => panic!("unexpected command: {command:?}"),
            }
        });

        let window_count = query_window_count().expect("window count");
        assert_eq!(window_count, 3);
        let _ = responder.join();

        shutdown_test_bridge(lifecycle, proxy, handle);
    }

    #[test]
    fn quit_dispatches_quit_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, proxy, handle) = init_test_bridge();

        quit().expect("quit command");
        let envelope = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("command envelope");

        match envelope.command {
            AppCommand::Quit => {}
            command => panic!("unexpected command: {command:?}"),
        }

        shutdown_test_bridge(lifecycle, proxy, handle);
    }
}
