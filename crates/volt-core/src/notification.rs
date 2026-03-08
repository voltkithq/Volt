use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("notification failed: {0}")]
    Send(String),
}

/// Configuration for a native notification.
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// Notification title.
    pub title: String,
    /// Notification body text.
    pub body: Option<String>,
    /// Path to an icon file (optional).
    pub icon: Option<String>,
}

/// Show a native OS notification.
pub fn show_notification(config: &NotificationConfig) -> Result<(), NotificationError> {
    let mut notification = notify_rust::Notification::new();
    notification.summary(&config.title);

    if let Some(ref body) = config.body {
        notification.body(body);
    }

    if let Some(ref icon) = config.icon {
        notification.icon(icon);
    }

    notification
        .show()
        .map_err(|e| NotificationError::Send(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_config_full() {
        let config = NotificationConfig {
            title: "Update Available".to_string(),
            body: Some("Version 2.0 is ready.".to_string()),
            icon: Some("/path/to/icon.png".to_string()),
        };
        assert_eq!(config.title, "Update Available");
        assert_eq!(config.body.as_deref(), Some("Version 2.0 is ready."));
        assert_eq!(config.icon.as_deref(), Some("/path/to/icon.png"));
    }

    #[test]
    fn test_notification_config_minimal() {
        let config = NotificationConfig {
            title: "Hello".to_string(),
            body: None,
            icon: None,
        };
        assert_eq!(config.title, "Hello");
        assert!(config.body.is_none());
        assert!(config.icon.is_none());
    }

    #[test]
    fn test_notification_config_clone() {
        let config = NotificationConfig {
            title: "Clone".to_string(),
            body: Some("body".to_string()),
            icon: None,
        };
        let cloned = config.clone();
        assert_eq!(cloned.title, "Clone");
        assert_eq!(cloned.body.as_deref(), Some("body"));
    }

    #[test]
    fn test_notification_config_debug() {
        let config = NotificationConfig {
            title: "Debug".to_string(),
            body: None,
            icon: None,
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("Debug"));
        assert!(debug.contains("NotificationConfig"));
    }

    #[test]
    fn test_notification_error_send_display() {
        let e = NotificationError::Send("dbus unavailable".to_string());
        let msg = e.to_string();
        assert!(msg.contains("notification failed"));
        assert!(msg.contains("dbus unavailable"));
    }
}
