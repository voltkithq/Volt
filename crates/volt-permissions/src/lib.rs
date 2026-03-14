use std::collections::HashSet;
use thiserror::Error;

/// Errors related to the permission system.
#[derive(Error, Debug)]
pub enum PermissionError {
    #[error("undeclared capability: '{0}' is not listed in the app's permissions")]
    UndeclaredCapability(String),
}

/// Capability-based permissions that must be declared in volt.config.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    Clipboard,
    Notification,
    Dialog,
    FileSystem,
    Database,
    Menu,
    Shell,
    Http,
    GlobalShortcut,
    Tray,
    SecureStorage,
}

impl Permission {
    /// Parse a permission from its string name (as used in volt.config.ts).
    pub fn from_str_name(name: &str) -> Option<Self> {
        match name {
            "clipboard" => Some(Self::Clipboard),
            "notification" => Some(Self::Notification),
            "dialog" => Some(Self::Dialog),
            "fs" => Some(Self::FileSystem),
            "db" => Some(Self::Database),
            "menu" => Some(Self::Menu),
            "shell" => Some(Self::Shell),
            "http" => Some(Self::Http),
            "globalShortcut" => Some(Self::GlobalShortcut),
            "tray" => Some(Self::Tray),
            "secureStorage" => Some(Self::SecureStorage),
            _ => None,
        }
    }

    /// Get the string name of this permission.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Notification => "notification",
            Self::Dialog => "dialog",
            Self::FileSystem => "fs",
            Self::Database => "db",
            Self::Menu => "menu",
            Self::Shell => "shell",
            Self::Http => "http",
            Self::GlobalShortcut => "globalShortcut",
            Self::Tray => "tray",
            Self::SecureStorage => "secureStorage",
        }
    }
}

/// Capability guard that checks permissions before allowing API access.
/// Permissions are loaded at app startup from volt.config.ts and are immutable.
pub struct CapabilityGuard {
    granted: HashSet<Permission>,
}

impl CapabilityGuard {
    /// Create a new capability guard with the given set of granted permissions.
    pub fn new(permissions: HashSet<Permission>) -> Self {
        Self {
            granted: permissions,
        }
    }

    /// Create a capability guard from a list of permission name strings.
    pub fn from_names(names: &[String]) -> Self {
        let granted = names
            .iter()
            .filter_map(|name| Permission::from_str_name(name))
            .collect();
        Self { granted }
    }

    /// Check if a permission has been granted. Returns error if not.
    pub fn check(&self, permission: Permission) -> Result<(), PermissionError> {
        if self.granted.contains(&permission) {
            Ok(())
        } else {
            Err(PermissionError::UndeclaredCapability(
                permission.as_str().to_string(),
            ))
        }
    }

    /// Check if a permission has been granted (boolean).
    pub fn has(&self, permission: Permission) -> bool {
        self.granted.contains(&permission)
    }

    /// Get all granted permissions.
    pub fn granted_permissions(&self) -> &HashSet<Permission> {
        &self.granted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        let guard =
            CapabilityGuard::from_names(&["clipboard".to_string(), "notification".to_string()]);

        assert!(guard.check(Permission::Clipboard).is_ok());
        assert!(guard.check(Permission::Notification).is_ok());
        assert!(guard.check(Permission::Shell).is_err());
        assert!(guard.check(Permission::FileSystem).is_err());
        assert!(guard.check(Permission::Database).is_err());
    }

    #[test]
    fn test_default_deny() {
        let guard = CapabilityGuard::new(HashSet::new());
        assert!(guard.check(Permission::Clipboard).is_err());
        assert!(guard.check(Permission::Tray).is_err());
    }

    #[test]
    fn test_permission_from_str() {
        assert_eq!(
            Permission::from_str_name("clipboard"),
            Some(Permission::Clipboard)
        );
        assert_eq!(
            Permission::from_str_name("fs"),
            Some(Permission::FileSystem)
        );
        assert_eq!(Permission::from_str_name("unknown"), None);
    }

    #[test]
    fn test_has_method() {
        let guard = CapabilityGuard::from_names(&["tray".to_string()]);
        assert!(guard.has(Permission::Tray));
        assert!(!guard.has(Permission::Shell));
    }

    #[test]
    fn test_all_permissions_roundtrip() {
        let all_names = [
            "clipboard",
            "notification",
            "dialog",
            "fs",
            "db",
            "menu",
            "shell",
            "http",
            "globalShortcut",
            "tray",
            "secureStorage",
        ];
        for name in &all_names {
            let perm = Permission::from_str_name(name)
                .unwrap_or_else(|| panic!("Failed to parse permission: {name}"));
            assert_eq!(perm.as_str(), *name);
        }
    }

    #[test]
    fn test_from_names_ignores_invalid() {
        let guard = CapabilityGuard::from_names(&[
            "clipboard".to_string(),
            "INVALID".to_string(),
            "tray".to_string(),
            "bogus".to_string(),
        ]);
        assert!(guard.has(Permission::Clipboard));
        assert!(guard.has(Permission::Tray));
        assert_eq!(guard.granted_permissions().len(), 2);
    }

    #[test]
    fn test_from_names_empty() {
        let guard = CapabilityGuard::from_names(&[]);
        assert_eq!(guard.granted_permissions().len(), 0);
        assert!(!guard.has(Permission::Clipboard));
    }

    #[test]
    fn test_granted_permissions_returns_all() {
        let guard = CapabilityGuard::from_names(&[
            "clipboard".to_string(),
            "notification".to_string(),
            "fs".to_string(),
        ]);
        let granted = guard.granted_permissions();
        assert_eq!(granted.len(), 3);
        assert!(granted.contains(&Permission::Clipboard));
        assert!(granted.contains(&Permission::Notification));
        assert!(granted.contains(&Permission::FileSystem));
    }

    #[test]
    fn test_permission_error_display() {
        let e = PermissionError::UndeclaredCapability("clipboard".into());
        let msg = e.to_string();
        assert!(msg.contains("clipboard"));
        assert!(msg.contains("undeclared"));
    }

    #[test]
    fn test_permission_from_str_all_invalid() {
        assert!(Permission::from_str_name("").is_none());
        assert!(Permission::from_str_name("Clipboard").is_none());
        assert!(Permission::from_str_name("CLIPBOARD").is_none());
        assert!(Permission::from_str_name("filesystem").is_none());
        assert!(Permission::from_str_name("global-shortcut").is_none());
    }

    #[test]
    fn test_permission_equality_and_hash() {
        let mut set = HashSet::new();
        set.insert(Permission::Clipboard);
        set.insert(Permission::Clipboard);
        assert_eq!(set.len(), 1);

        set.insert(Permission::Shell);
        assert_eq!(set.len(), 2);
    }
}
