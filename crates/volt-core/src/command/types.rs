use crate::menu::MenuItemConfig;
use std::sync::mpsc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct TrayCommandConfig {
    pub tooltip: Option<String>,
    pub icon_rgba: Option<Vec<u8>>,
    pub icon_width: u32,
    pub icon_height: u32,
}

/// Commands from NAPI into the tao event loop thread.
pub enum AppCommand {
    CloseWindow {
        js_id: String,
    },
    ShowWindow {
        js_id: String,
    },
    FocusWindow {
        js_id: String,
    },
    MaximizeWindow {
        js_id: String,
    },
    MinimizeWindow {
        js_id: String,
    },
    RestoreWindow {
        js_id: String,
    },
    EvaluateScript {
        js_id: String,
        script: String,
    },
    EmitEvent {
        js_window_id: Option<String>,
        event_name: String,
        data: serde_json::Value,
    },
    GetWindowCount {
        reply: mpsc::Sender<u32>,
    },
    IpcMessage {
        js_window_id: String,
        raw: String,
    },
    SetAppMenu {
        items: Vec<MenuItemConfig>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    RegisterShortcut {
        accelerator: String,
        reply: mpsc::Sender<Result<u32, String>>,
    },
    UnregisterShortcut {
        accelerator: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    UnregisterAllShortcuts {
        reply: mpsc::Sender<Result<(), String>>,
    },
    CreateTray {
        config: TrayCommandConfig,
        reply: mpsc::Sender<Result<String, String>>,
    },
    SetTrayTooltip {
        tooltip: String,
        reply: mpsc::Sender<Result<(), String>>,
    },
    SetTrayVisible {
        visible: bool,
        reply: mpsc::Sender<Result<(), String>>,
    },
    DestroyTray {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Quit,
}

impl std::fmt::Debug for AppCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CloseWindow { js_id } => write!(f, "CloseWindow({js_id})"),
            Self::ShowWindow { js_id } => write!(f, "ShowWindow({js_id})"),
            Self::FocusWindow { js_id } => write!(f, "FocusWindow({js_id})"),
            Self::MaximizeWindow { js_id } => write!(f, "MaximizeWindow({js_id})"),
            Self::MinimizeWindow { js_id } => write!(f, "MinimizeWindow({js_id})"),
            Self::RestoreWindow { js_id } => write!(f, "RestoreWindow({js_id})"),
            Self::EvaluateScript { js_id, .. } => write!(f, "EvaluateScript({js_id})"),
            Self::EmitEvent {
                js_window_id,
                event_name,
                ..
            } => {
                if let Some(js_id) = js_window_id {
                    write!(f, "EmitEvent({event_name} -> {js_id})")
                } else {
                    write!(f, "EmitEvent({event_name} -> broadcast)")
                }
            }
            Self::GetWindowCount { .. } => write!(f, "GetWindowCount"),
            Self::IpcMessage { js_window_id, .. } => write!(f, "IpcMessage({js_window_id})"),
            Self::SetAppMenu { .. } => write!(f, "SetAppMenu"),
            Self::RegisterShortcut { accelerator, .. } => {
                write!(f, "RegisterShortcut({accelerator})")
            }
            Self::UnregisterShortcut { accelerator, .. } => {
                write!(f, "UnregisterShortcut({accelerator})")
            }
            Self::UnregisterAllShortcuts { .. } => write!(f, "UnregisterAllShortcuts"),
            Self::CreateTray { .. } => write!(f, "CreateTray"),
            Self::SetTrayTooltip { .. } => write!(f, "SetTrayTooltip"),
            Self::SetTrayVisible { visible, .. } => write!(f, "SetTrayVisible({visible})"),
            Self::DestroyTray { .. } => write!(f, "DestroyTray"),
            Self::Quit => write!(f, "Quit"),
        }
    }
}

/// Internal command envelope carrying observability metadata.
pub struct CommandEnvelope {
    pub trace_id: u64,
    pub enqueued_at: Instant,
    pub command: AppCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandObservabilitySnapshot {
    pub commands_sent: u64,
    pub commands_processed: u64,
    pub commands_failed: u64,
}
