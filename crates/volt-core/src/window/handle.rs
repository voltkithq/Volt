use tao::dpi::{LogicalPosition, LogicalSize};
use tao::window::{Window, WindowId};

/// Handle wrapping a tao Window with convenience methods.
pub struct WindowHandle {
    window: Window,
}

impl WindowHandle {
    /// Create a new WindowHandle from a raw tao Window.
    pub fn new(window: Window) -> Self {
        Self { window }
    }

    /// Get the window's unique identifier.
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Get a reference to the underlying tao Window.
    pub fn inner(&self) -> &Window {
        &self.window
    }

    /// Set the window title.
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    /// Get the window title.
    pub fn title(&self) -> String {
        self.window.title()
    }

    /// Set the window size in logical pixels.
    pub fn set_size(&self, width: f64, height: f64) {
        self.window.set_inner_size(LogicalSize::new(width, height));
    }

    /// Get the window's inner size in logical pixels.
    pub fn inner_size(&self) -> (f64, f64) {
        let size = self.window.inner_size();
        let scale = self.window.scale_factor();
        (size.width as f64 / scale, size.height as f64 / scale)
    }

    /// Set the window position in logical pixels.
    pub fn set_position(&self, x: f64, y: f64) {
        self.window.set_outer_position(LogicalPosition::new(x, y));
    }

    /// Set whether the window is resizable.
    pub fn set_resizable(&self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    /// Set whether the window is always on top.
    pub fn set_always_on_top(&self, always_on_top: bool) {
        self.window.set_always_on_top(always_on_top);
    }

    /// Set window visibility.
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    /// Request focus for the window.
    pub fn focus(&self) {
        self.window.set_focus();
    }

    /// Maximize the window.
    pub fn maximize(&self) {
        self.window.set_maximized(true);
    }

    /// Minimize the window.
    pub fn minimize(&self) {
        self.window.set_minimized(true);
    }

    /// Restore the window from maximized/minimized state.
    pub fn restore(&self) {
        self.window.set_maximized(false);
        self.window.set_minimized(false);
    }

    /// Hide the window without destroying native resources.
    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    /// Legacy alias for `hide()`.
    /// Native close/destroy is managed by the app event loop command path.
    pub fn close(&self) {
        self.hide();
    }
}
