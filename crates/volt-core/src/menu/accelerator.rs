use muda::{PredefinedMenuItem, accelerator::Accelerator};

use super::MenuError;

pub(super) fn parse_menu_accelerator(raw: Option<&str>) -> Result<Option<Accelerator>, MenuError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }

    if let Ok(parsed) = raw.parse::<Accelerator>() {
        return Ok(Some(parsed));
    }

    // Normalize Electron-style CmdOrCtrl token for muda parser.
    let normalized = if cfg!(target_os = "macos") {
        raw.replace("CmdOrCtrl", "Cmd")
    } else {
        raw.replace("CmdOrCtrl", "Ctrl")
    };

    normalized
        .parse::<Accelerator>()
        .map(Some)
        .map_err(|e| MenuError::Operation(format!("Invalid menu accelerator '{raw}': {e}")))
}

pub(super) fn predefined_item_from_role(role: &str) -> Option<PredefinedMenuItem> {
    match role {
        "quit" => Some(PredefinedMenuItem::quit(None)),
        "copy" => Some(PredefinedMenuItem::copy(None)),
        "cut" => Some(PredefinedMenuItem::cut(None)),
        "paste" => Some(PredefinedMenuItem::paste(None)),
        "selectAll" | "select-all" => Some(PredefinedMenuItem::select_all(None)),
        "undo" => Some(PredefinedMenuItem::undo(None)),
        "redo" => Some(PredefinedMenuItem::redo(None)),
        "minimize" => Some(PredefinedMenuItem::minimize(None)),
        "separator" => Some(PredefinedMenuItem::separator()),
        _ => None,
    }
}
