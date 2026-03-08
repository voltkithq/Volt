use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during window operations.
#[derive(Error, Debug)]
pub enum WindowError {
    #[error("failed to build window: {0}")]
    Build(String),

    #[error("window operation failed: {0}")]
    Operation(String),
}

/// Configuration for creating a new window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title.
    #[serde(default = "default_title")]
    pub title: String,

    /// Window width in logical pixels.
    #[serde(default = "default_width")]
    pub width: f64,

    /// Window height in logical pixels.
    #[serde(default = "default_height")]
    pub height: f64,

    /// Minimum window width.
    pub min_width: Option<f64>,

    /// Minimum window height.
    pub min_height: Option<f64>,

    /// Maximum window width.
    pub max_width: Option<f64>,

    /// Maximum window height.
    pub max_height: Option<f64>,

    /// Whether the window is resizable.
    #[serde(default = "default_true")]
    pub resizable: bool,

    /// Whether the window has OS decorations (title bar, borders).
    #[serde(default = "default_true")]
    pub decorations: bool,

    /// Whether the window background is transparent.
    #[serde(default)]
    pub transparent: bool,

    /// Whether the window is always on top.
    #[serde(default)]
    pub always_on_top: bool,

    /// Whether the window starts maximized.
    #[serde(default)]
    pub maximized: bool,

    /// Whether the window starts visible.
    #[serde(default = "default_true")]
    pub visible: bool,

    /// Initial X position (if not set, OS decides).
    pub x: Option<f64>,

    /// Initial Y position (if not set, OS decides).
    pub y: Option<f64>,
}

fn default_title() -> String {
    String::from("Volt")
}

fn default_width() -> f64 {
    800.0
}

fn default_height() -> f64 {
    600.0
}

fn default_true() -> bool {
    true
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            width: default_width(),
            height: default_height(),
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            resizable: true,
            decorations: true,
            transparent: false,
            always_on_top: false,
            maximized: false,
            visible: true,
            x: None,
            y: None,
        }
    }
}
