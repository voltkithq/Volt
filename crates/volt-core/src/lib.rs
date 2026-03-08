pub mod app;
pub mod clipboard;
pub mod command;
pub mod dialog;
pub mod embed;
pub mod fs;
pub mod global_shortcut;
pub mod ipc;
pub mod menu;
pub mod notification;
pub mod permissions;
pub mod security;
pub mod shell;
pub mod tray;
pub mod updater;
pub mod webview;
pub mod window;

pub use app::{App, AppConfig, AppEvent};
pub use clipboard::{ClipboardError, ImageData};
pub use command::{
    AppCommand, CommandBridgeError, CommandObservabilitySnapshot, command_observability_snapshot,
    send_command, send_query,
};
pub use dialog::{MessageDialogOptions, OpenDialogOptions, SaveDialogOptions};
pub use embed::AssetBundle;
pub use fs::{FileInfo, FsError};
pub use global_shortcut::{ShortcutError, ShortcutManager};
pub use ipc::{IpcRegistry, IpcRequest, IpcResponse};
pub use menu::MenuItemConfig;
pub use notification::{NotificationConfig, NotificationError};
pub use permissions::{CapabilityGuard, Permission};
pub use shell::ShellError;
pub use tray::TrayConfig;
pub use updater::{UpdateConfig, UpdateError, UpdateInfo};
pub use webview::WebViewConfig;
pub use window::{WindowConfig, WindowHandle};
