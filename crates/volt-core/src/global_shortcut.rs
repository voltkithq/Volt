use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShortcutError {
    #[error("failed to register shortcut '{0}': {1}")]
    Register(String, String),

    #[error("failed to unregister shortcut '{0}': {1}")]
    Unregister(String, String),

    #[error("shortcut operation failed: {0}")]
    Operation(String),
}

/// Global shortcut management.
/// Shortcuts are managed through tao's GlobalShortcutManager.
/// This module provides the type-safe tracking interface.
///
/// Note: tao's GlobalShortcutManager requires access to the event loop,
/// so actual registration happens through the App struct.
pub struct ShortcutManager {
    registered: Vec<String>,
}

impl ShortcutManager {
    /// Create a new shortcut manager.
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
        }
    }

    /// Track a registered shortcut accelerator string.
    pub fn track(&mut self, accelerator: &str) {
        if !self.registered.contains(&accelerator.to_string()) {
            self.registered.push(accelerator.to_string());
        }
    }

    /// Remove a tracked shortcut.
    pub fn untrack(&mut self, accelerator: &str) {
        self.registered.retain(|s| s != accelerator);
    }

    /// Get all tracked accelerator strings.
    pub fn all(&self) -> &[String] {
        &self.registered
    }

    /// Clear all tracked shortcuts.
    pub fn clear(&mut self) {
        self.registered.clear();
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let mgr = ShortcutManager::new();
        assert!(mgr.all().is_empty());
    }

    #[test]
    fn test_default_empty() {
        let mgr = ShortcutManager::default();
        assert!(mgr.all().is_empty());
    }

    #[test]
    fn test_track_shortcut() {
        let mut mgr = ShortcutManager::new();
        mgr.track("CmdOrCtrl+C");
        assert_eq!(mgr.all().len(), 1);
        assert_eq!(mgr.all()[0], "CmdOrCtrl+C");
    }

    #[test]
    fn test_track_multiple() {
        let mut mgr = ShortcutManager::new();
        mgr.track("CmdOrCtrl+C");
        mgr.track("CmdOrCtrl+V");
        mgr.track("CmdOrCtrl+X");
        assert_eq!(mgr.all().len(), 3);
    }

    #[test]
    fn test_track_duplicate_ignored() {
        let mut mgr = ShortcutManager::new();
        mgr.track("CmdOrCtrl+C");
        mgr.track("CmdOrCtrl+C");
        assert_eq!(mgr.all().len(), 1);
    }

    #[test]
    fn test_untrack_shortcut() {
        let mut mgr = ShortcutManager::new();
        mgr.track("CmdOrCtrl+C");
        mgr.track("CmdOrCtrl+V");
        mgr.untrack("CmdOrCtrl+C");
        assert_eq!(mgr.all().len(), 1);
        assert_eq!(mgr.all()[0], "CmdOrCtrl+V");
    }

    #[test]
    fn test_untrack_nonexistent() {
        let mut mgr = ShortcutManager::new();
        mgr.untrack("does-not-exist");
        assert!(mgr.all().is_empty());
    }

    #[test]
    fn test_clear() {
        let mut mgr = ShortcutManager::new();
        mgr.track("A");
        mgr.track("B");
        mgr.track("C");
        assert_eq!(mgr.all().len(), 3);
        mgr.clear();
        assert!(mgr.all().is_empty());
    }

    #[test]
    fn test_shortcut_error_register_display() {
        let e = ShortcutError::Register("CmdOrCtrl+X".into(), "already taken".into());
        let msg = e.to_string();
        assert!(msg.contains("CmdOrCtrl+X"));
        assert!(msg.contains("already taken"));
    }

    #[test]
    fn test_shortcut_error_unregister_display() {
        let e = ShortcutError::Unregister("CmdOrCtrl+Z".into(), "not found".into());
        let msg = e.to_string();
        assert!(msg.contains("CmdOrCtrl+Z"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_shortcut_error_operation_display() {
        let e = ShortcutError::Operation("something broke".into());
        assert!(e.to_string().contains("something broke"));
    }
}
