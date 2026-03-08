use super::bridge::{BridgeDispatch, build_bridge_dispatches};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use volt_core::app::{App, AppConfig};
use volt_core::webview::WebViewConfig;
use volt_core::window::WindowConfig;

const RUNTIME_START_TIMEOUT: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT_QUIT_WAIT: Duration = Duration::from_secs(2);
const STARTUP_TIMEOUT_JOIN_WAIT: Duration = Duration::from_secs(3);

pub(super) struct RuntimeContext {
    pub(super) native_thread: JoinHandle<Result<(), String>>,
    pub(super) bridge_thread: JoinHandle<()>,
}

pub(super) fn cleanup_finished_runtime(
    runtime: &Arc<Mutex<Option<RuntimeContext>>>,
) -> napi::Result<()> {
    let mut runtime_guard = runtime
        .lock()
        .map_err(|e| napi::Error::from_reason(format!("Runtime lock poisoned: {e}")))?;

    let finished = runtime_guard
        .as_ref()
        .is_some_and(|ctx| ctx.native_thread.is_finished() && ctx.bridge_thread.is_finished());

    if !finished {
        return Ok(());
    }

    if let Some(ctx) = runtime_guard.take() {
        let _ = ctx.native_thread.join();
        let _ = ctx.bridge_thread.join();
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub(super) fn wait_for_runtime_start(
    native_thread: &JoinHandle<Result<(), String>>,
) -> Result<(), String> {
    wait_for_runtime_start_with_probe(
        RUNTIME_START_TIMEOUT,
        Duration::from_millis(10),
        volt_core::command::is_running,
        native_thread,
    )
}

pub(super) fn wait_for_runtime_start_with_probe(
    timeout: Duration,
    poll_interval: Duration,
    mut is_running: impl FnMut() -> bool,
    native_thread: &JoinHandle<Result<(), String>>,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if is_running() {
            return Ok(());
        }
        if native_thread.is_finished() {
            break;
        }
        thread::sleep(poll_interval);
    }

    if is_running() {
        return Ok(());
    }

    Err("Native runtime failed to start within timeout".to_string())
}

#[cfg(not(target_os = "macos"))]
pub(super) fn spawn_startup_timeout_cleanup(
    native_thread: JoinHandle<Result<(), String>>,
    bridge_thread: JoinHandle<()>,
    dispatch_tx: mpsc::Sender<BridgeDispatch>,
) {
    let _ = thread::Builder::new()
        .name("volt-runtime-timeout-cleanup".to_string())
        .spawn(move || {
            request_quit_after_startup_timeout(STARTUP_TIMEOUT_QUIT_WAIT);

            let join_deadline = Instant::now() + STARTUP_TIMEOUT_JOIN_WAIT;
            while Instant::now() < join_deadline {
                if native_thread.is_finished() {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }

            if native_thread.is_finished() {
                let _ = native_thread.join();
                let _ = bridge_thread.join();
                return;
            }

            // If startup hangs, force-stop bridge dispatch and detach native handle.
            detach_runtime_after_startup_timeout(&dispatch_tx);
            let _ = bridge_thread.join();
        });
}

#[cfg(not(target_os = "macos"))]
fn request_quit_after_startup_timeout(wait: Duration) {
    let _ = request_quit_after_startup_timeout_with_probe(
        wait,
        Duration::from_millis(10),
        volt_core::command::is_running,
        || {
            volt_core::command::send_command(volt_core::command::AppCommand::Quit)
                .map(|_| ())
                .map_err(|_| ())
        },
    );
}

#[cfg(not(target_os = "macos"))]
pub(super) fn request_quit_after_startup_timeout_with_probe(
    wait: Duration,
    poll_interval: Duration,
    mut is_running: impl FnMut() -> bool,
    mut send_quit: impl FnMut() -> Result<(), ()>,
) -> bool {
    let deadline = Instant::now() + wait;
    while Instant::now() < deadline {
        if is_running() {
            return send_quit().is_ok();
        }
        thread::sleep(poll_interval);
    }
    false
}

#[cfg(not(target_os = "macos"))]
fn detach_runtime_after_startup_timeout(dispatch_tx: &mpsc::Sender<BridgeDispatch>) {
    detach_runtime_after_startup_timeout_with(dispatch_tx, volt_core::command::shutdown_bridge);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn detach_runtime_after_startup_timeout_with(
    dispatch_tx: &mpsc::Sender<BridgeDispatch>,
    mut shutdown_bridge: impl FnMut(),
) {
    let _ = dispatch_tx.send(BridgeDispatch::RuntimeStopped);
    // If we must abandon the native thread, clear global command bridge state so future runs are not blocked.
    shutdown_bridge();
    tracing::warn!(
        "native runtime did not stop after startup-timeout cleanup request; detached native thread"
    );
}

pub(super) fn run_native_loop(
    app_config: AppConfig,
    windows: Vec<(WindowConfig, WebViewConfig, String)>,
    dispatch_tx: mpsc::Sender<BridgeDispatch>,
) -> Result<(), String> {
    let mut app = App::new(app_config).map_err(|e| format!("Failed to create app: {e}"))?;
    let mut tao_to_js: HashMap<String, String> = HashMap::new();

    for (window_config, webview_config, js_window_id) in windows {
        let tao_id = app
            .create_window_with_js_id(window_config, webview_config, js_window_id.clone())
            .map_err(|e| format!("Failed to create window: {e}"))?;
        tao_to_js.insert(window_key(&tao_id), js_window_id);
    }

    app.run(move |event| {
        for dispatch in build_bridge_dispatches(event, &mut tao_to_js) {
            let _ = dispatch_tx.send(dispatch);
        }
    })
    .map_err(|e| format!("Event loop error: {e}"))
}

fn window_key(id: &impl std::hash::Hash) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    format!("native-window-{:016x}", hasher.finish())
}
