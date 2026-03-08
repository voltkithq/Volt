use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};

use tao::event_loop::{EventLoopBuilder, EventLoopProxy};
use volt_core::app::AppEvent;
use volt_core::command::{self, CommandEnvelope};

pub fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Returns a shared event loop proxy. Only one GTK event loop can exist per
/// process on Linux, so all tests must share the same instance. The event
/// loop is leaked to keep it alive for the process lifetime.
pub fn shared_event_loop_proxy() -> EventLoopProxy<AppEvent> {
    static PROXY: OnceLock<EventLoopProxy<AppEvent>> = OnceLock::new();
    PROXY
        .get_or_init(|| {
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
            std::mem::forget(event_loop);
            proxy
        })
        .clone()
}

pub fn init_test_bridge() -> (
    mpsc::Receiver<CommandEnvelope>,
    command::BridgeLifecycle,
    EventLoopProxy<AppEvent>,
) {
    command::shutdown_bridge();
    let proxy = shared_event_loop_proxy();
    let registration = command::init_bridge(proxy.clone()).expect("bridge init");
    (registration.receiver, registration.lifecycle, proxy)
}

pub fn shutdown_test_bridge(lifecycle: command::BridgeLifecycle) {
    lifecycle.shutdown();
}
