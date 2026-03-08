use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use super::accelerator::{parse_menu_accelerator, predefined_item_from_role};
use super::{MenuError, MenuItemConfig};

static MENU_EVENT_ID_MAP: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn menu_event_id_map() -> &'static Mutex<HashMap<String, String>> {
    MENU_EVENT_ID_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn update_menu_event_id_map(mapping: HashMap<String, String>) {
    if let Ok(mut guard) = menu_event_id_map().lock() {
        *guard = mapping;
    }
}

/// Resolve a native internal menu ID to a configured stable menu ID, if one exists.
pub fn resolve_menu_event_id(internal_id: &str) -> Option<String> {
    menu_event_id_map()
        .lock()
        .ok()
        .and_then(|guard| guard.get(internal_id).cloned())
}

/// Build a muda Menu from a list of item configurations.
pub fn build_menu(items: &[MenuItemConfig]) -> Result<Menu, MenuError> {
    let menu = Menu::new();
    let mut id_mapping = HashMap::new();

    for item_config in items {
        add_menu_item(&menu, item_config, &mut id_mapping)?;
    }

    update_menu_event_id_map(id_mapping);
    Ok(menu)
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

#[cfg(test)]
pub(super) fn menu_event_id_map_snapshot() -> HashMap<String, String> {
    menu_event_id_map()
        .lock()
        .expect("menu id mapping lock")
        .clone()
}
