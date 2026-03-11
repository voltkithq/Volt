use napi_derive::napi;
use serde_json::Value;
use volt_core::command::{AppCommand, send_query};
use volt_core::menu::{self, MenuItemConfig};
use volt_core::permissions::Permission;

use crate::permissions::require_permission;

/// JavaScript-facing menu builder.
#[napi]
pub struct VoltMenu {
    items: Vec<MenuItemConfig>,
}

#[napi]
impl VoltMenu {
    /// Create a new menu from a JSON template array.
    #[napi(constructor)]
    pub fn new(template: Value) -> napi::Result<Self> {
        require_permission(Permission::Menu)?;
        let items = parse_menu_template(&template)?;
        Ok(Self { items })
    }

    /// Build and set this menu as the application menu bar.
    /// Must be called from the main thread (event loop context).
    #[napi]
    pub fn set_as_app_menu(&self) -> napi::Result<()> {
        require_permission(Permission::Menu)?;
        // Validate template shape early for clearer JS-side errors.
        let (_menu, _id_map) = menu::build_menu(&self.items)
            .map_err(|e| napi::Error::from_reason(format!("Failed to build menu: {e}")))?;

        send_query(|reply| AppCommand::SetAppMenu {
            items: self.items.clone(),
            reply,
        })
        .map_err(|err| napi::Error::from_reason(err.to_string()))?
        .map_err(napi::Error::from_reason)?;
        Ok(())
    }

    /// Get the number of top-level menu items.
    #[napi]
    pub fn item_count(&self) -> u32 {
        self.items.len() as u32
    }
}

/// Parse a JSON value into a list of MenuItemConfig.
fn parse_menu_template(template: &Value) -> napi::Result<Vec<MenuItemConfig>> {
    let items = template
        .as_array()
        .ok_or_else(|| napi::Error::from_reason("Menu template must be an array"))?;

    items.iter().map(parse_menu_item).collect()
}

fn parse_menu_item(value: &Value) -> napi::Result<MenuItemConfig> {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let label = value
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let accelerator = value
        .get("accelerator")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let enabled = value
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let item_type = value
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("normal")
        .to_string();

    let role = value
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let submenu = if let Some(sub_items) = value.get("submenu").and_then(|v| v.as_array()) {
        sub_items
            .iter()
            .map(parse_menu_item)
            .collect::<napi::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    Ok(MenuItemConfig {
        id,
        label,
        accelerator,
        enabled,
        item_type,
        role,
        submenu,
    })
}
