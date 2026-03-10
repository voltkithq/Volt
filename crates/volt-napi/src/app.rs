use crate::permissions::configure_permissions;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use volt_core::app::AppConfig;
use volt_core::webview::WebViewConfig;
use volt_core::window::WindowConfig;

mod bridge;
mod config;
mod runtime;
#[cfg(test)]
mod tests;

use bridge::{BridgeDispatch, spawn_bridge_thread};
use config::{parse_webview_config, parse_window_config};
use runtime::{RuntimeContext, cleanup_finished_runtime, run_native_loop};
#[cfg(not(target_os = "macos"))]
use runtime::{spawn_startup_timeout_cleanup, wait_for_runtime_start};

/// JavaScript-facing application class.
#[napi]
pub struct VoltApp {
    name: String,
    devtools: bool,
    windows_to_create: Arc<Mutex<Vec<(WindowConfig, WebViewConfig, String)>>>,
    event_callbacks: Arc<Mutex<Vec<ThreadsafeFunction<String>>>>,
    runtime: Arc<Mutex<Option<RuntimeContext>>>,
}

#[napi]
impl VoltApp {
    /// Create a new VoltApp from a JSON configuration object.
    #[napi(constructor)]
    pub fn new(config: Value) -> napi::Result<Self> {
        configure_permissions(&config)?;

        let name = config
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Volt App")
            .to_string();

        let devtools = config
            .get("devtools")
            .and_then(|v| v.as_bool())
            .unwrap_or(cfg!(debug_assertions));
        crate::logging::init_logging(if devtools { "debug" } else { "warn" });

        Ok(Self {
            name,
            devtools,
            windows_to_create: Arc::new(Mutex::new(Vec::new())),
            event_callbacks: Arc::new(Mutex::new(Vec::new())),
            runtime: Arc::new(Mutex::new(None)),
        })
    }

    /// Queue a window for creation. The window will be created when `run()` is called.
    #[napi]
    pub fn create_window(&self, config: Value) -> napi::Result<()> {
        if volt_core::command::is_running() {
            return Err(napi::Error::from_reason(
                "createWindow() after run() is not supported in v0.1; create windows before run()",
            ));
        }

        let window_config = parse_window_config(&config);
        let webview_config = parse_webview_config(&config);
        let js_window_id = config
            .get("jsId")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .unwrap_or_else(volt_core::app::allocate_js_window_id);

        let mut windows = self
            .windows_to_create
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock poisoned: {e}")))?;
        windows.push((window_config, webview_config, js_window_id));

        Ok(())
    }

    /// Register an event callback. The callback receives a JSON string with event details.
    #[napi]
    pub fn on_event(&self, callback: ThreadsafeFunction<String>) -> napi::Result<()> {
        let mut callbacks = self
            .event_callbacks
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock poisoned: {e}")))?;
        callbacks.push(callback);
        Ok(())
    }

    /// Run the application event loop runtime.
    ///
    /// On non-macOS platforms this starts a split runtime:
    /// - native loop on a dedicated thread
    /// - Node bridge callback dispatch on a dedicated thread
    ///
    /// On macOS, tao/wry main-thread requirements keep the native loop on the
    /// current thread.
    #[napi]
    pub fn run(&self) -> napi::Result<()> {
        cleanup_finished_runtime(&self.runtime)?;
        if volt_core::command::is_running() {
            return Err(napi::Error::from_reason(
                "native runtime is already running for this process",
            ));
        }

        #[cfg(target_os = "macos")]
        {
            self.run_macos_main_thread()
        }

        #[cfg(not(target_os = "macos"))]
        {
            self.run_split_runtime()
        }
    }
}

impl VoltApp {
    #[cfg(target_os = "macos")]
    fn run_macos_main_thread(&self) -> napi::Result<()> {
        let app_config = self.app_config();
        let windows = self.take_windows_to_create()?;
        let callbacks = Arc::clone(&self.event_callbacks);
        let (dispatch_tx, dispatch_rx) = mpsc::channel();
        let bridge_thread = spawn_bridge_thread(callbacks, dispatch_rx)?;

        let run_result = run_native_loop(app_config, windows, dispatch_tx.clone());
        let _ = dispatch_tx.send(BridgeDispatch::RuntimeStopped);
        let _ = bridge_thread.join();

        run_result.map_err(napi::Error::from_reason)
    }

    #[cfg(not(target_os = "macos"))]
    fn run_split_runtime(&self) -> napi::Result<()> {
        let app_config = self.app_config();
        let windows = self.take_windows_to_create()?;
        let callbacks = Arc::clone(&self.event_callbacks);
        let (dispatch_tx, dispatch_rx) = mpsc::channel();

        let bridge_thread = spawn_bridge_thread(callbacks, dispatch_rx)?;
        let native_tx = dispatch_tx.clone();
        let startup_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let startup_error_writer = Arc::clone(&startup_error);
        let native_thread = thread::Builder::new()
            .name("volt-native-loop".to_string())
            .spawn(move || {
                let result = run_native_loop(app_config, windows, native_tx.clone());
                if let Err(err) = &result {
                    if let Ok(mut guard) = startup_error_writer.lock() {
                        *guard = Some(err.clone());
                    }
                    let payload = json!({
                        "type": "runtime-error",
                        "error": err,
                    });
                    let _ = native_tx.send(BridgeDispatch::EventJson(payload.to_string()));
                }
                let _ = native_tx.send(BridgeDispatch::RuntimeStopped);
                result
            })
            .map_err(|e| napi::Error::from_reason(format!("Failed to spawn native thread: {e}")))?;

        if let Err(err) = wait_for_runtime_start(&native_thread) {
            // Try to extract the actual error from the native thread.
            let detail = startup_error.lock().ok().and_then(|guard| guard.clone());
            let message = match detail {
                Some(native_err) => format!("{err}: {native_err}"),
                None => err,
            };
            spawn_startup_timeout_cleanup(native_thread, bridge_thread, dispatch_tx);
            return Err(napi::Error::from_reason(message));
        }

        let mut runtime = self
            .runtime
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Runtime lock poisoned: {e}")))?;
        *runtime = Some(RuntimeContext {
            native_thread,
            bridge_thread,
        });

        Ok(())
    }

    fn app_config(&self) -> AppConfig {
        AppConfig {
            name: self.name.clone(),
            devtools: self.devtools,
        }
    }

    fn take_windows_to_create(&self) -> napi::Result<Vec<(WindowConfig, WebViewConfig, String)>> {
        let mut guard = self
            .windows_to_create
            .lock()
            .map_err(|e| napi::Error::from_reason(format!("Lock poisoned: {e}")))?;
        Ok(std::mem::take(&mut *guard))
    }
}
