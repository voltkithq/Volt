use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DialogError {
    #[error("dialog operation failed: {0}")]
    Operation(String),
}

/// Filter for file dialogs (e.g., "Images" -> ["png", "jpg"]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// Options for the open file dialog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenDialogOptions {
    /// Dialog title.
    #[serde(default)]
    pub title: Option<String>,

    /// Default starting directory.
    #[serde(default)]
    pub default_path: Option<String>,

    /// File type filters.
    #[serde(default)]
    pub filters: Vec<FileFilter>,

    /// Allow selecting multiple files.
    #[serde(default)]
    pub multiple: bool,

    /// Allow selecting directories instead of files.
    #[serde(default)]
    pub directory: bool,
}

/// Options for the save file dialog.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveDialogOptions {
    /// Dialog title.
    #[serde(default)]
    pub title: Option<String>,

    /// Default file path/name.
    #[serde(default)]
    pub default_path: Option<String>,

    /// File type filters.
    #[serde(default)]
    pub filters: Vec<FileFilter>,
}

/// Options for message box dialogs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDialogOptions {
    /// Message box type: "info", "warning", "error".
    #[serde(default = "default_info")]
    pub dialog_type: String,

    /// Dialog title.
    #[serde(default)]
    pub title: String,

    /// Dialog message.
    pub message: String,

    /// Button labels.
    /// Currently used only to select button count (0/1 => OK, 2 => OK/Cancel).
    /// Custom button text is not supported by rfd and is ignored.
    #[serde(default)]
    pub buttons: Vec<String>,
}

fn default_info() -> String {
    "info".to_string()
}

/// Show an open file dialog. Returns selected file paths, or empty if cancelled.
pub fn show_open_dialog(options: &OpenDialogOptions) -> Vec<PathBuf> {
    let mut builder = if options.directory {
        rfd::FileDialog::new().set_title(options.title.as_deref().unwrap_or("Open Folder"))
    } else {
        rfd::FileDialog::new().set_title(options.title.as_deref().unwrap_or("Open File"))
    };

    if let Some(ref default_path) = options.default_path {
        builder = builder.set_directory(default_path);
    }

    for filter in &options.filters {
        let exts: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        builder = builder.add_filter(&filter.name, &exts);
    }

    if options.directory {
        builder.pick_folder().map(|p| vec![p]).unwrap_or_default()
    } else if options.multiple {
        builder.pick_files().unwrap_or_default()
    } else {
        builder.pick_file().map(|p| vec![p]).unwrap_or_default()
    }
}

/// Show a save file dialog. Returns the selected path, or None if cancelled.
pub fn show_save_dialog(options: &SaveDialogOptions) -> Option<PathBuf> {
    let mut builder =
        rfd::FileDialog::new().set_title(options.title.as_deref().unwrap_or("Save File"));

    if let Some(ref default_path) = options.default_path {
        let path = PathBuf::from(default_path);
        if let Some(dir) = path.parent() {
            builder = builder.set_directory(dir);
        }
        if let Some(name) = path.file_name() {
            builder = builder.set_file_name(name.to_string_lossy().as_ref());
        }
    }

    for filter in &options.filters {
        let exts: Vec<&str> = filter.extensions.iter().map(|s| s.as_str()).collect();
        builder = builder.add_filter(&filter.name, &exts);
    }

    builder.save_file()
}

/// Show a message dialog. Returns true if user confirmed (OK/Yes), false otherwise.
pub fn show_message_dialog(options: &MessageDialogOptions) -> bool {
    let level = match options.dialog_type.as_str() {
        "warning" => rfd::MessageLevel::Warning,
        "error" => rfd::MessageLevel::Error,
        _ => rfd::MessageLevel::Info,
    };

    let buttons = if options.buttons.is_empty() {
        rfd::MessageButtons::Ok
    } else if options.buttons.len() == 2 {
        rfd::MessageButtons::OkCancel
    } else {
        rfd::MessageButtons::Ok
    };

    let dialog = rfd::MessageDialog::new()
        .set_level(level)
        .set_title(&options.title)
        .set_description(&options.message)
        .set_buttons(buttons);

    matches!(
        dialog.show(),
        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FileFilter ─────────────────────────────────────────────────

    #[test]
    fn test_file_filter_serde_roundtrip() {
        let filter = FileFilter {
            name: "Images".to_string(),
            extensions: vec!["png".to_string(), "jpg".to_string(), "gif".to_string()],
        };
        let json = serde_json::to_string(&filter).unwrap();
        let restored: FileFilter = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Images");
        assert_eq!(restored.extensions.len(), 3);
        assert_eq!(restored.extensions[0], "png");
    }

    #[test]
    fn test_file_filter_debug() {
        let filter = FileFilter {
            name: "Docs".to_string(),
            extensions: vec!["pdf".to_string()],
        };
        let debug = format!("{:?}", filter);
        assert!(debug.contains("Docs"));
        assert!(debug.contains("pdf"));
    }

    // ── OpenDialogOptions ──────────────────────────────────────────

    #[test]
    fn test_open_dialog_options_default() {
        let opts = OpenDialogOptions::default();
        assert!(opts.title.is_none());
        assert!(opts.default_path.is_none());
        assert!(opts.filters.is_empty());
        assert!(!opts.multiple);
        assert!(!opts.directory);
    }

    #[test]
    fn test_open_dialog_options_serde_roundtrip() {
        let opts = OpenDialogOptions {
            title: Some("Open Image".to_string()),
            default_path: Some("/home/user".to_string()),
            filters: vec![FileFilter {
                name: "Images".to_string(),
                extensions: vec!["png".to_string()],
            }],
            multiple: true,
            directory: false,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let restored: OpenDialogOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.title.as_deref(), Some("Open Image"));
        assert!(restored.multiple);
        assert!(!restored.directory);
        assert_eq!(restored.filters.len(), 1);
    }

    #[test]
    fn test_open_dialog_options_serde_empty_json() {
        let opts: OpenDialogOptions = serde_json::from_str("{}").unwrap();
        assert!(opts.title.is_none());
        assert!(!opts.multiple);
        assert!(!opts.directory);
    }

    #[test]
    fn test_open_dialog_options_directory_mode() {
        let opts: OpenDialogOptions = serde_json::from_str(r#"{"directory":true}"#).unwrap();
        assert!(opts.directory);
    }

    // ── SaveDialogOptions ──────────────────────────────────────────

    #[test]
    fn test_save_dialog_options_default() {
        let opts = SaveDialogOptions::default();
        assert!(opts.title.is_none());
        assert!(opts.default_path.is_none());
        assert!(opts.filters.is_empty());
    }

    #[test]
    fn test_save_dialog_options_serde_roundtrip() {
        let opts = SaveDialogOptions {
            title: Some("Save As".to_string()),
            default_path: Some("/home/user/doc.pdf".to_string()),
            filters: vec![FileFilter {
                name: "PDF".to_string(),
                extensions: vec!["pdf".to_string()],
            }],
        };
        let json = serde_json::to_string(&opts).unwrap();
        let restored: SaveDialogOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.title.as_deref(), Some("Save As"));
        assert_eq!(restored.default_path.as_deref(), Some("/home/user/doc.pdf"));
        assert_eq!(restored.filters.len(), 1);
    }

    // ── MessageDialogOptions ───────────────────────────────────────

    #[test]
    fn test_message_dialog_options_default_type() {
        let opts: MessageDialogOptions = serde_json::from_str(r#"{"message":"Hello"}"#).unwrap();
        assert_eq!(opts.dialog_type, "info");
        assert_eq!(opts.title, "");
        assert_eq!(opts.message, "Hello");
        assert!(opts.buttons.is_empty());
    }

    #[test]
    fn test_message_dialog_options_serde_roundtrip() {
        let opts = MessageDialogOptions {
            dialog_type: "warning".to_string(),
            title: "Confirm".to_string(),
            message: "Are you sure?".to_string(),
            buttons: vec!["OK".to_string(), "Cancel".to_string()],
        };
        let json = serde_json::to_string(&opts).unwrap();
        let restored: MessageDialogOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.dialog_type, "warning");
        assert_eq!(restored.title, "Confirm");
        assert_eq!(restored.message, "Are you sure?");
        assert_eq!(restored.buttons.len(), 2);
    }

    #[test]
    fn test_message_dialog_all_types() {
        for dialog_type in &["info", "warning", "error"] {
            let json = format!(r#"{{"dialog_type":"{}","message":"test"}}"#, dialog_type);
            let opts: MessageDialogOptions = serde_json::from_str(&json).unwrap();
            assert_eq!(opts.dialog_type, *dialog_type);
        }
    }

    // ── DialogError ────────────────────────────────────────────────

    #[test]
    fn test_dialog_error_operation_display() {
        let e = DialogError::Operation("user cancelled".to_string());
        let msg = e.to_string();
        assert!(msg.contains("dialog operation"));
        assert!(msg.contains("user cancelled"));
    }
}
