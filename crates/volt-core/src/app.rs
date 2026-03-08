use crate::embed::AssetBundle;
use crate::webview::{WebViewConfig, create_webview};
use crate::window::{WindowConfig, WindowHandle, create_window};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tao::event_loop::{EventLoop, EventLoopBuilder, EventLoopProxy};
use thiserror::Error;

mod command_handling;
mod event_loop;
#[cfg(test)]
mod tests;
mod window_management;

type WindowStore = HashMap<tao::window::WindowId, (WindowHandle, wry::WebView)>;
type WindowStateStore = HashMap<tao::window::WindowId, WindowLifecycleState>;

static NEXT_WINDOW_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate the next stable JS-facing window ID.
pub fn allocate_js_window_id() -> String {
    let id = NEXT_WINDOW_ID.fetch_add(1, Ordering::Relaxed);
    format!("window-{id}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowLifecycleState {
    Active,
    Closing,
    Closed,
}

/// Errors that can occur during application lifecycle.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("failed to create event loop: {0}")]
    EventLoopCreation(String),

    #[error("failed to create window: {0}")]
    WindowCreation(String),

    #[error("failed to create webview: {0}")]
    WebViewCreation(String),

    #[error("event loop already consumed")]
    EventLoopConsumed,

    #[error("application error: {0}")]
    Generic(String),
}

/// Custom events for cross-thread communication via EventLoopProxy.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Request to create a new window with the given config.
    CreateWindow {
        window_config: Box<WindowConfig>,
        webview_config: Box<WebViewConfig>,
        js_window_id: Option<String>,
    },
    /// Request to close a specific window by ID.
    CloseWindow(tao::window::WindowId),
    /// Request to quit the application.
    Quit,
    /// Execute a script in a specific window's webview.
    EvaluateScript {
        window_id: tao::window::WindowId,
        script: String,
    },
    /// Internal wake event so the loop drains pending AppCommands.
    ProcessCommands,
    /// IPC message from WebView -> Node bridge.
    IpcMessage { js_window_id: String, raw: String },
    /// Menu click notification.
    MenuEvent { menu_id: String },
    /// Global shortcut activation notification.
    ShortcutTriggered { id: u32 },
    /// Tray click notification.
    TrayEvent { tray_id: String },
}

/// Application-level configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Display name of the application.
    pub name: String,
    /// Whether to enable dev tools (default: cfg!(debug_assertions)).
    pub devtools: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: String::from("Volt App"),
            devtools: cfg!(debug_assertions),
        }
    }
}

/// Main application struct managing the event loop, windows, and webviews.
pub struct App {
    config: AppConfig,
    event_loop: Option<EventLoop<AppEvent>>,
    proxy: EventLoopProxy<AppEvent>,
    windows: WindowStore,
    window_states: WindowStateStore,
    /// JS browser window ID -> tao window ID mapping.
    js_to_tao: HashMap<String, tao::window::WindowId>,
    /// Reverse mapping to emit events with JS IDs.
    tao_to_js: HashMap<tao::window::WindowId, String>,
    asset_bundle: Option<Arc<AssetBundle>>,
}

impl App {
    /// Create a new application with the given configuration.
    pub fn new(config: AppConfig) -> Result<Self, AppError> {
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

        Ok(Self {
            config,
            event_loop: Some(event_loop),
            proxy,
            windows: HashMap::new(),
            window_states: HashMap::new(),
            js_to_tao: HashMap::new(),
            tao_to_js: HashMap::new(),
            asset_bundle: None,
        })
    }

    /// Set the asset bundle for the `volt://` custom protocol.
    /// Call this before `create_window()` so windows serve embedded assets.
    pub fn set_asset_bundle(&mut self, bundle: AssetBundle) {
        self.asset_bundle = Some(Arc::new(bundle));
    }

    /// Get a proxy for sending events to the event loop from other threads.
    pub fn proxy(&self) -> EventLoopProxy<AppEvent> {
        self.proxy.clone()
    }

    /// Get the application configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Create a window and its associated webview.
    /// Must be called before `run()` for initial windows.
    pub fn create_window(
        &mut self,
        window_config: WindowConfig,
        webview_config: WebViewConfig,
    ) -> Result<tao::window::WindowId, AppError> {
        self.create_window_with_js_id(window_config, webview_config, allocate_js_window_id())
    }

    /// Create a window using a caller-provided JS window ID.
    pub fn create_window_with_js_id(
        &mut self,
        window_config: WindowConfig,
        webview_config: WebViewConfig,
        js_window_id: String,
    ) -> Result<tao::window::WindowId, AppError> {
        let event_loop = self
            .event_loop
            .as_ref()
            .ok_or(AppError::EventLoopConsumed)?;

        let window_handle = create_window(event_loop, &window_config)
            .map_err(|e| AppError::WindowCreation(e.to_string()))?;

        let webview = create_webview(
            window_handle.inner(),
            &webview_config,
            self.config.devtools,
            self.asset_bundle.clone(),
            js_window_id.clone(),
        )
        .map_err(|e| AppError::WebViewCreation(e.to_string()))?;

        let window_id = window_handle.id();
        self.windows.insert(window_id, (window_handle, webview));
        window_management::set_window_active(&mut self.window_states, window_id);
        self.js_to_tao.insert(js_window_id.clone(), window_id);
        self.tao_to_js.insert(window_id, js_window_id);
        window_management::debug_assert_window_invariants(
            &self.windows,
            &self.js_to_tao,
            &self.tao_to_js,
            &self.window_states,
        );
        Ok(window_id)
    }

    /// Run the application event loop. This consumes the App and blocks until all windows are closed.
    pub fn run<F>(self, on_event: F) -> Result<(), AppError>
    where
        F: FnMut(&AppEvent) + 'static,
    {
        event_loop::run_event_loop(self, on_event)
    }
}
