use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MenuError {
    #[error("failed to create menu: {0}")]
    Creation(String),

    #[error("menu operation failed: {0}")]
    Operation(String),
}

/// Menu item configuration (mirrors Electron's MenuItem options).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItemConfig {
    /// Stable menu item ID for callback dispatch.
    #[serde(default)]
    pub id: Option<String>,

    /// Display label.
    pub label: String,

    /// Keyboard accelerator (e.g., "CmdOrCtrl+C").
    #[serde(default)]
    pub accelerator: Option<String>,

    /// Whether the item is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Menu item type: "normal", "separator", "submenu".
    #[serde(default = "default_type")]
    pub item_type: String,

    /// Role-based items: "quit", "copy", "paste", "selectAll", etc.
    #[serde(default)]
    pub role: Option<String>,

    /// Submenu items (only for type "submenu").
    #[serde(default)]
    pub submenu: Vec<MenuItemConfig>,
}

fn default_true() -> bool {
    true
}

fn default_type() -> String {
    "normal".to_string()
}
