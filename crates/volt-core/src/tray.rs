use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrayError {
    #[error("failed to create tray: {0}")]
    Creation(String),

    #[error("failed to set tray icon: {0}")]
    SetIcon(String),

    #[error("tray operation failed: {0}")]
    Operation(String),
}

/// Configuration for creating a system tray icon.
#[derive(Debug, Clone)]
pub struct TrayConfig {
    /// Tooltip text shown on hover.
    pub tooltip: Option<String>,
    /// PNG icon bytes.
    pub icon: Option<Vec<u8>>,
    /// Icon width (required if icon is provided).
    pub icon_width: u32,
    /// Icon height (required if icon is provided).
    pub icon_height: u32,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            tooltip: None,
            icon: None,
            icon_width: 32,
            icon_height: 32,
        }
    }
}

/// Handle to a system tray icon.
pub struct TrayHandle {
    tray: tray_icon::TrayIcon,
}

impl TrayHandle {
    /// Create a new tray icon with the given configuration.
    pub fn new(config: &TrayConfig) -> Result<Self, TrayError> {
        let mut builder = tray_icon::TrayIconBuilder::new();

        if let Some(ref tooltip) = config.tooltip {
            builder = builder.with_tooltip(tooltip);
        }

        if let Some(ref icon_data) = config.icon {
            let icon = tray_icon::Icon::from_rgba(
                icon_data.clone(),
                config.icon_width,
                config.icon_height,
            )
            .map_err(|e| TrayError::SetIcon(e.to_string()))?;
            builder = builder.with_icon(icon);
        }

        let tray = builder
            .build()
            .map_err(|e| TrayError::Creation(e.to_string()))?;

        Ok(Self { tray })
    }

    /// Set the tray tooltip.
    pub fn set_tooltip(&self, tooltip: &str) -> Result<(), TrayError> {
        self.tray
            .set_tooltip(Some(tooltip))
            .map_err(|e| TrayError::Operation(e.to_string()))
    }

    /// Set the tray icon from RGBA data.
    pub fn set_icon(&self, rgba: Vec<u8>, width: u32, height: u32) -> Result<(), TrayError> {
        let icon = tray_icon::Icon::from_rgba(rgba, width, height)
            .map_err(|e| TrayError::SetIcon(e.to_string()))?;
        self.tray
            .set_icon(Some(icon))
            .map_err(|e| TrayError::Operation(e.to_string()))
    }

    /// Set the tray visibility.
    pub fn set_visible(&self, visible: bool) -> Result<(), TrayError> {
        self.tray
            .set_visible(visible)
            .map_err(|e| TrayError::Operation(e.to_string()))
    }

    /// Get the stable tray icon ID used for event dispatch.
    pub fn id(&self) -> &str {
        self.tray.id().as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_config_default() {
        let config = TrayConfig::default();
        assert!(config.tooltip.is_none());
        assert!(config.icon.is_none());
        assert_eq!(config.icon_width, 32);
        assert_eq!(config.icon_height, 32);
    }

    #[test]
    fn test_tray_config_custom() {
        let config = TrayConfig {
            tooltip: Some("My App".to_string()),
            icon: Some(vec![0, 0, 0, 255]),
            icon_width: 16,
            icon_height: 16,
        };
        assert_eq!(config.tooltip.as_deref(), Some("My App"));
        assert!(config.icon.is_some());
        assert_eq!(config.icon_width, 16);
        assert_eq!(config.icon_height, 16);
    }

    #[test]
    fn test_tray_config_clone() {
        let config = TrayConfig {
            tooltip: Some("Test".to_string()),
            ..TrayConfig::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.tooltip.as_deref(), Some("Test"));
        assert_eq!(cloned.icon_width, 32);
    }

    #[test]
    fn test_tray_config_debug() {
        let config = TrayConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("TrayConfig"));
    }

    #[test]
    fn test_tray_error_creation_display() {
        let e = TrayError::Creation("no system tray support".to_string());
        let msg = e.to_string();
        assert!(msg.contains("create tray"));
        assert!(msg.contains("no system tray support"));
    }

    #[test]
    fn test_tray_error_set_icon_display() {
        let e = TrayError::SetIcon("invalid rgba".to_string());
        let msg = e.to_string();
        assert!(msg.contains("icon"));
        assert!(msg.contains("invalid rgba"));
    }

    #[test]
    fn test_tray_error_operation_display() {
        let e = TrayError::Operation("tooltip too long".to_string());
        let msg = e.to_string();
        assert!(msg.contains("operation"));
        assert!(msg.contains("tooltip too long"));
    }
}
