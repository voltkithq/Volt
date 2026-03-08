use std::path::{Path, PathBuf};

use boa_engine::object::builtins::JsFunction;
use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use serde_json::Value;
use volt_core::command::{self, AppCommand, TrayCommandConfig};
use volt_core::fs as core_fs;
use volt_core::permissions::Permission;

use super::{
    bind_native_event_off, bind_native_event_on, fs_base_dir, js_error, native_function_module,
    normalize_single_event_name, promise_from_result, require_permission_message, value_to_json,
};

const TRAY_CLICK_EVENT: &str = "tray:click";
const DEFAULT_TRAY_ICON_SIZE: u32 = 32;

fn normalize_tray_event_name(event_name: String) -> Result<&'static str, String> {
    normalize_single_event_name("tray", event_name, "click", TRAY_CLICK_EVENT)
}

fn load_icon_rgba(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let image = image::open(path)
        .map_err(|error| format!("failed to load tray icon '{}': {error}", path.display()))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

fn resolve_icon_path(icon_path: &str) -> Result<PathBuf, String> {
    require_permission_message(Permission::FileSystem)?;
    let base = fs_base_dir()?;
    core_fs::safe_resolve(&base, icon_path)
        .map_err(|error| format!("failed to resolve tray icon path: {error}"))
}

fn parse_tray_config(
    options: Option<JsValue>,
    context: &mut Context,
) -> Result<TrayCommandConfig, String> {
    let Some(options) = options else {
        return Ok(TrayCommandConfig {
            tooltip: None,
            icon_rgba: None,
            icon_width: DEFAULT_TRAY_ICON_SIZE,
            icon_height: DEFAULT_TRAY_ICON_SIZE,
        });
    };

    let value = value_to_json(options, context)?;
    parse_tray_config_value(&value)
}

fn parse_tray_config_value(value: &Value) -> Result<TrayCommandConfig, String> {
    let mut config = TrayCommandConfig {
        tooltip: None,
        icon_rgba: None,
        icon_width: DEFAULT_TRAY_ICON_SIZE,
        icon_height: DEFAULT_TRAY_ICON_SIZE,
    };

    let object = value
        .as_object()
        .ok_or_else(|| "tray options must be an object".to_string())?;

    if let Some(tooltip) = object.get("tooltip") {
        config.tooltip = Some(
            tooltip
                .as_str()
                .ok_or_else(|| "tray option 'tooltip' must be a string".to_string())?
                .to_string(),
        );
    }

    if let Some(icon) = object.get("icon") {
        let icon_path = icon
            .as_str()
            .ok_or_else(|| "tray option 'icon' must be a string path".to_string())?;
        if icon_path.trim().is_empty() {
            return Err("tray option 'icon' must not be empty".to_string());
        }
        let resolved_icon_path = resolve_icon_path(icon_path)?;
        let (rgba, width, height) = load_icon_rgba(&resolved_icon_path)?;
        config.icon_rgba = Some(rgba);
        config.icon_width = width;
        config.icon_height = height;
    }

    Ok(config)
}

fn create_tray(config: TrayCommandConfig) -> Result<(), String> {
    command::send_query(|reply| AppCommand::CreateTray { config, reply })
        .map_err(|error| format!("failed to send tray create command: {error}"))?
        .map(|_| ())
        .map_err(|error| format!("failed to create tray: {error}"))
}

fn set_tray_tooltip(tooltip: String) -> Result<(), String> {
    require_permission_message(Permission::Tray)?;
    command::send_query(|reply| AppCommand::SetTrayTooltip { tooltip, reply })
        .map_err(|error| format!("failed to send tray tooltip command: {error}"))?
        .map_err(|error| format!("failed to set tray tooltip: {error}"))
}

fn set_tray_visible(visible: bool) -> Result<(), String> {
    require_permission_message(Permission::Tray)?;
    command::send_query(|reply| AppCommand::SetTrayVisible { visible, reply })
        .map_err(|error| format!("failed to send tray visibility command: {error}"))?
        .map_err(|error| format!("failed to set tray visibility: {error}"))
}

fn destroy_tray() -> Result<(), String> {
    require_permission_message(Permission::Tray)?;
    command::send_query(|reply| AppCommand::DestroyTray { reply })
        .map_err(|error| format!("failed to send tray destroy command: {error}"))?
        .map_err(|error| format!("failed to destroy tray: {error}"))
}

fn create(options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission_message(Permission::Tray)?;
        let config = parse_tray_config(options, context)?;
        create_tray(config)
    })();
    promise_from_result(context, result).into()
}

fn set_tooltip(tooltip: String) -> JsResult<()> {
    set_tray_tooltip(tooltip).map_err(|error| js_error("volt:tray", "setTooltip", error))
}

fn set_visible(visible: bool) -> JsResult<()> {
    set_tray_visible(visible).map_err(|error| js_error("volt:tray", "setVisible", error))
}

fn destroy() -> JsResult<()> {
    destroy_tray().map_err(|error| js_error("volt:tray", "destroy", error))
}

fn on(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_on(
        context,
        "volt:tray",
        Permission::Tray,
        event_name,
        handler,
        normalize_tray_event_name,
    )
}

fn off(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_off(
        context,
        "volt:tray",
        Permission::Tray,
        event_name,
        handler,
        normalize_tray_event_name,
    )
}

pub fn build_module(context: &mut Context) -> Module {
    let create = create.into_js_function_copied(context);
    let set_tooltip = set_tooltip.into_js_function_copied(context);
    let set_visible = set_visible.into_js_function_copied(context);
    let destroy = destroy.into_js_function_copied(context);
    let on = on.into_js_function_copied(context);
    let off = off.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("create", create),
            ("setTooltip", set_tooltip),
            ("setVisible", set_visible),
            ("destroy", destroy),
            ("on", on),
            ("off", off),
        ],
    )
}

#[cfg(test)]
mod tests;
