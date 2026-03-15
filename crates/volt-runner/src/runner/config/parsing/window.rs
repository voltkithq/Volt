use serde_json::Value;
use volt_core::window::WindowConfig;

pub(super) fn parse_window_config(parsed: &Value) -> WindowConfig {
    let (icon_rgba, icon_width, icon_height) = parsed
        .get("icon")
        .and_then(Value::as_str)
        .and_then(load_icon_rgba)
        .map(|(data, w, h)| (Some(data), w, h))
        .unwrap_or((None, 0, 0));

    WindowConfig {
        title: parsed
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Volt")
            .to_string(),
        width: parsed.get("width").and_then(Value::as_f64).unwrap_or(800.0),
        height: parsed
            .get("height")
            .and_then(Value::as_f64)
            .unwrap_or(600.0),
        min_width: parsed.get("minWidth").and_then(Value::as_f64),
        min_height: parsed.get("minHeight").and_then(Value::as_f64),
        max_width: parsed.get("maxWidth").and_then(Value::as_f64),
        max_height: parsed.get("maxHeight").and_then(Value::as_f64),
        resizable: parsed
            .get("resizable")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        decorations: parsed
            .get("decorations")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        transparent: parsed
            .get("transparent")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        always_on_top: parsed
            .get("alwaysOnTop")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        maximized: parsed
            .get("maximized")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        visible: parsed
            .get("visible")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        x: parsed.get("x").and_then(Value::as_f64),
        y: parsed.get("y").and_then(Value::as_f64),
        icon_rgba,
        icon_width,
        icon_height,
    }
}

fn load_icon_rgba(path: &str) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some((img.into_raw(), w, h))
}
