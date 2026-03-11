use std::collections::HashMap;

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use super::accelerator::{parse_menu_accelerator, predefined_item_from_role};
use super::{MenuError, MenuItemConfig};

/// Build a muda Menu from a list of item configurations.
///
/// Returns the built menu together with a mapping from muda's internal menu-item
/// IDs to the caller-supplied stable IDs.  The caller owns this mapping and can
/// use it to resolve menu events without relying on any process-wide global state.
pub fn build_menu(items: &[MenuItemConfig]) -> Result<(Menu, HashMap<String, String>), MenuError> {
    let menu = Menu::new();
    let mut id_mapping = HashMap::new();

    for item_config in items {
        add_menu_item(&menu, item_config, &mut id_mapping)?;
    }

    Ok((menu, id_mapping))
}

fn add_menu_item(
    menu: &Menu,
    config: &MenuItemConfig,
    id_mapping: &mut HashMap<String, String>,
) -> Result<(), MenuError> {
    match config.item_type.as_str() {
        "separator" => {
            menu.append(&PredefinedMenuItem::separator())
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
        "submenu" => {
            let custom_id = config.id.clone();
            let submenu = if let Some(id) = custom_id.clone() {
                Submenu::with_id(id, &config.label, config.enabled)
            } else {
                Submenu::new(&config.label, config.enabled)
            };

            if let Some(id) = custom_id {
                id_mapping.insert(submenu.id().0.to_string(), id);
            }

            for sub_item in &config.submenu {
                add_submenu_item(&submenu, sub_item, id_mapping)?;
            }

            menu.append(&submenu)
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
        _ => {
            if let Some(ref role) = config.role
                && let Some(predefined) = predefined_item_from_role(role)
            {
                if let Some(custom_id) = &config.id {
                    id_mapping.insert(predefined.id().0.to_string(), custom_id.clone());
                }
                menu.append(&predefined)
                    .map_err(|e| MenuError::Operation(e.to_string()))?;
                return Ok(());
            }

            let accelerator = parse_menu_accelerator(config.accelerator.as_deref())?;
            let custom_id = config.id.clone();
            let item = if let Some(id) = custom_id.clone() {
                MenuItem::with_id(id, &config.label, config.enabled, accelerator)
            } else {
                MenuItem::new(&config.label, config.enabled, accelerator)
            };

            if let Some(id) = custom_id {
                id_mapping.insert(item.id().0.to_string(), id);
            }

            menu.append(&item)
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
    }

    Ok(())
}

fn add_submenu_item(
    submenu: &Submenu,
    config: &MenuItemConfig,
    id_mapping: &mut HashMap<String, String>,
) -> Result<(), MenuError> {
    match config.item_type.as_str() {
        "separator" => {
            submenu
                .append(&PredefinedMenuItem::separator())
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
        "submenu" => {
            let custom_id = config.id.clone();
            let nested = if let Some(id) = custom_id.clone() {
                Submenu::with_id(id, &config.label, config.enabled)
            } else {
                Submenu::new(&config.label, config.enabled)
            };

            if let Some(id) = custom_id {
                id_mapping.insert(nested.id().0.to_string(), id);
            }

            for sub_item in &config.submenu {
                add_submenu_item(&nested, sub_item, id_mapping)?;
            }

            submenu
                .append(&nested)
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
        _ => {
            if let Some(ref role) = config.role
                && let Some(predefined) = predefined_item_from_role(role)
            {
                if let Some(custom_id) = &config.id {
                    id_mapping.insert(predefined.id().0.to_string(), custom_id.clone());
                }
                submenu
                    .append(&predefined)
                    .map_err(|e| MenuError::Operation(e.to_string()))?;
                return Ok(());
            }

            let accelerator = parse_menu_accelerator(config.accelerator.as_deref())?;
            let custom_id = config.id.clone();
            let item = if let Some(id) = custom_id.clone() {
                MenuItem::with_id(id, &config.label, config.enabled, accelerator)
            } else {
                MenuItem::new(&config.label, config.enabled, accelerator)
            };

            if let Some(id) = custom_id {
                id_mapping.insert(item.id().0.to_string(), id);
            }

            submenu
                .append(&item)
                .map_err(|e| MenuError::Operation(e.to_string()))?;
        }
    }

    Ok(())
}

/// Poll for menu events. Call this in the event loop.
pub fn check_menu_event() -> Option<MenuEvent> {
    MenuEvent::receiver().try_recv().ok()
}
