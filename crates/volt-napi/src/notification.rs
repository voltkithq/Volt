use napi_derive::napi;
use serde_json::Value;
use volt_core::notification::{self, NotificationConfig};
use volt_core::permissions::Permission;

use crate::permissions::require_permission;

/// Show a native OS notification.
/// Accepts a JSON object with `title` (required), `body` (optional), and `icon` (optional).
#[napi]
pub fn notification_show(options: Value) -> napi::Result<()> {
    require_permission(Permission::Notification)?;

    let title = options
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| napi::Error::from_reason("Notification requires a 'title' field"))?
        .to_string();

    let body = options
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let icon = options
        .get("icon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let config = NotificationConfig { title, body, icon };

    notification::show_notification(&config)
        .map_err(|e| napi::Error::from_reason(format!("Notification failed: {e}")))
}
