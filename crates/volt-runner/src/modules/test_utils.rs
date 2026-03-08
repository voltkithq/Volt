use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use volt_core::app::AppEvent;
use volt_core::command::{self, CommandEnvelope};

pub fn test_guard() -> std::sync::MutexGuard<'static, ()> {
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

pub fn init_test_bridge() -> (
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

pub fn shutdown_test_bridge(
    lifecycle: command::BridgeLifecycle,
    proxy: EventLoopProxy<AppEvent>,
    handle: JoinHandle<()>,
) {
    lifecycle.shutdown();
    let _ = proxy.send_event(AppEvent::Quit);
    let _ = handle.join();
}
