use boa_engine::object::builtins::JsFunction;
use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use serde_json::Value;
use volt_core::command::{self, AppCommand};
use volt_core::menu::MenuItemConfig;
use volt_core::permissions::Permission;

use super::{
    bind_native_event_off, bind_native_event_on, native_function_module,
    normalize_single_event_name, promise_from_result, require_permission_message, value_to_json,
};

const MENU_CLICK_EVENT: &str = "menu:click";

fn normalize_menu_event_name(event_name: String) -> Result<&'static str, String> {
    normalize_single_event_name("menu", event_name, "click", MENU_CLICK_EVENT)
}

fn parse_menu_template(template: &Value) -> Result<Vec<MenuItemConfig>, String> {
    let items = template
        .as_array()
        .ok_or_else(|| "menu template must be an array".to_string())?;
    items.iter().map(parse_menu_item).collect()
}

fn parse_menu_item(value: &Value) -> Result<MenuItemConfig, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "menu item must be an object".to_string())?;

    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let label = object
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let accelerator = object
        .get("accelerator")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let enabled = object
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let item_type = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("normal")
        .to_string();

    let role = object
        .get("role")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let submenu = if let Some(sub_items) = object.get("submenu") {
        let sub_items = sub_items
            .as_array()
            .ok_or_else(|| "menu item submenu must be an array".to_string())?;
        sub_items
            .iter()
            .map(parse_menu_item)
            .collect::<Result<Vec<_>, _>>()?
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

fn set_app_menu(items: Vec<MenuItemConfig>) -> Result<(), String> {
    command::send_query(|reply| AppCommand::SetAppMenu { items, reply })
        .map_err(|error| format!("failed to send app menu command: {error}"))?
        .map_err(|error| format!("failed to set app menu: {error}"))
}

fn ensure_menu_permission() -> Result<(), String> {
    require_permission_message(Permission::Menu)
}

fn set_app_menu_export(template: JsValue, context: &mut Context) -> JsValue {
    let result = (|| {
        ensure_menu_permission()?;
        let template = value_to_json(template, context)?;
        let items = parse_menu_template(&template)?;
        set_app_menu(items)
    })();

    promise_from_result(context, result).into()
}

fn on(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_on(
        context,
        "volt:menu",
        Permission::Menu,
        event_name,
        handler,
        normalize_menu_event_name,
    )
}

fn off(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_off(
        context,
        "volt:menu",
        Permission::Menu,
        event_name,
        handler,
        normalize_menu_event_name,
    )
}

pub fn build_module(context: &mut Context) -> Module {
    let set_app_menu_export = set_app_menu_export.into_js_function_copied(context);
    let on = on.into_js_function_copied(context);
    let off = off.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("setAppMenu", set_app_menu_export),
            ("on", on),
            ("off", off),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_utils::{init_test_bridge, shutdown_test_bridge, test_guard};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn parse_menu_template_supports_nested_submenus() {
        let parsed = parse_menu_template(&serde_json::json!([
            {
                "label": "File",
                "type": "submenu",
                "submenu": [
                    {
                        "id": "file-open",
                        "label": "Open",
                        "accelerator": "CmdOrCtrl+O",
                        "enabled": true
                    }
                ]
            }
        ]))
        .expect("menu template");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].item_type, "submenu");
        assert_eq!(parsed[0].submenu.len(), 1);
        assert_eq!(parsed[0].submenu[0].id.as_deref(), Some("file-open"));
    }

    #[test]
    fn normalize_menu_event_name_accepts_click_only() {
        assert_eq!(
            normalize_menu_event_name("click".to_string()),
            Ok("menu:click")
        );
        assert!(normalize_menu_event_name("".to_string()).is_err());
        assert!(normalize_menu_event_name("opened".to_string()).is_err());
    }

    #[test]
    fn set_app_menu_dispatches_command() {
        let _guard = test_guard();
        let (receiver, lifecycle, _proxy) = init_test_bridge();
        crate::modules::configure(crate::modules::ModuleConfig {
            fs_base_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            permissions: vec!["menu".to_string()],
            ..Default::default()
        })
        .expect("configure module permissions");

        let responder = thread::spawn(move || {
            let envelope = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("command envelope");
            match envelope.command {
                AppCommand::SetAppMenu { items, reply } => {
                    assert_eq!(items.len(), 1);
                    assert_eq!(items[0].id.as_deref(), Some("file-open"));
                    let _ = reply.send(Ok(()));
                }
                command => panic!("unexpected command: {command:?}"),
            }
        });

        set_app_menu(vec![MenuItemConfig {
            id: Some("file-open".to_string()),
            label: "Open".to_string(),
            accelerator: Some("CmdOrCtrl+O".to_string()),
            enabled: true,
            item_type: "normal".to_string(),
            role: None,
            submenu: Vec::new(),
        }])
        .expect("set app menu");

        let _ = responder.join();
        shutdown_test_bridge(lifecycle);
    }
}
