use super::*;
use std::sync::{Mutex, OnceLock};
use tao::event_loop::{EventLoopBuilder, EventLoopProxy};

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test guard lock")
}

fn test_event_loop_proxy() -> EventLoopProxy<crate::app::AppEvent> {
    // Reuse a single static event loop to avoid GTK D-Bus re-registration
    // errors on Linux (only one GTK Application per process is allowed).
    static PROXY: OnceLock<EventLoopProxy<crate::app::AppEvent>> = OnceLock::new();
    PROXY
        .get_or_init(|| {
            let mut builder = EventLoopBuilder::<crate::app::AppEvent>::with_user_event();
            #[cfg(target_os = "windows")]
            {
                use tao::platform::windows::EventLoopBuilderExtWindows;
                builder.with_any_thread(true);
            }
            #[cfg(target_os = "linux")]
            {
                use tao::platform::unix::EventLoopBuilderExtUnix;
                builder.with_any_thread(true);
            }
            let event_loop = builder.build();
            let proxy = event_loop.create_proxy();
            // Leak the event loop to keep GTK application alive for the process.
            std::mem::forget(event_loop);
            proxy
        })
        .clone()
}

#[test]
fn lifecycle_drop_clears_bridge() {
    let _guard = test_guard();
    shutdown_bridge();
    let registration = init_bridge(test_event_loop_proxy()).expect("bridge init");
    assert!(is_running());
    drop(registration);
    assert!(!is_running());
}

#[test]
fn double_init_is_rejected_while_active() {
    let _guard = test_guard();
    shutdown_bridge();
    let registration = init_bridge(test_event_loop_proxy()).expect("bridge init");
    let second = init_bridge(test_event_loop_proxy());
    assert!(second.is_err());
    drop(registration);
}

#[test]
fn explicit_shutdown_clears_bridge() {
    let _guard = test_guard();
    shutdown_bridge();
    let registration = init_bridge(test_event_loop_proxy()).expect("bridge init");
    assert!(is_running());
    registration.lifecycle.shutdown();
    assert!(!is_running());
}

#[test]
fn command_metrics_track_failed_send() {
    let _guard = test_guard();
    shutdown_bridge();
    let before = command_observability_snapshot().commands_failed;
    let _ = send_command(AppCommand::Quit);
    let after = command_observability_snapshot().commands_failed;
    assert!(after > before);
}

#[test]
fn observability_counters_reset_on_reinit() {
    let _guard = test_guard();
    shutdown_bridge();
    let registration = init_bridge(test_event_loop_proxy()).expect("bridge init");
    let _ = send_command(AppCommand::Quit);
    let sent_before = command_observability_snapshot().commands_sent;
    assert!(sent_before > 0);
    drop(registration);

    let _registration = init_bridge(test_event_loop_proxy()).expect("bridge re-init");
    let snapshot = command_observability_snapshot();
    assert_eq!(snapshot.commands_sent, 0);
    assert_eq!(snapshot.commands_processed, 0);
    assert_eq!(snapshot.commands_failed, 0);
}
