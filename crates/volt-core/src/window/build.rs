use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Icon, WindowBuilder};

use super::{WindowConfig, WindowError, WindowHandle};

/// Create a tao Window from a WindowConfig.
pub fn create_window<T: 'static>(
    event_loop: &EventLoopWindowTarget<T>,
    config: &WindowConfig,
) -> Result<WindowHandle, WindowError> {
    let mut builder = WindowBuilder::new()
        .with_title(&config.title)
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.decorations)
        .with_transparent(config.transparent)
        .with_always_on_top(config.always_on_top)
        .with_maximized(config.maximized)
        .with_visible(config.visible);

    if let Some(ref icon_data) = config.icon_rgba
        && let Ok(icon) = Icon::from_rgba(icon_data.clone(), config.icon_width, config.icon_height)
    {
        builder = builder.with_window_icon(Some(icon));
    }

    if let Some((min_w, min_h)) =
        resolve_optional_pair(config.min_width, config.min_height, 0.0, 0.0)
    {
        builder = builder.with_min_inner_size(LogicalSize::new(min_w, min_h));
    }

    if let Some((max_w, max_h)) =
        resolve_optional_pair(config.max_width, config.max_height, f64::MAX, f64::MAX)
    {
        builder = builder.with_max_inner_size(LogicalSize::new(max_w, max_h));
    }

    let partial_position = match (config.x, config.y) {
        (Some(x), Some(y)) => {
            builder = builder.with_position(LogicalPosition::new(x, y));
            None
        }
        (x, y) if x.is_some() || y.is_some() => Some((x, y)),
        _ => None,
    };

    let window = builder
        .build(event_loop)
        .map_err(|error| WindowError::Build(error.to_string()))?;

    if let Some((x, y)) = partial_position
        && let Ok(current_position) = window.outer_position()
    {
        let logical_position = current_position.to_logical::<f64>(window.scale_factor());
        let (current_x, current_y) = (logical_position.x, logical_position.y);
        window.set_outer_position(LogicalPosition::new(
            x.unwrap_or(current_x),
            y.unwrap_or(current_y),
        ));
    }

    Ok(WindowHandle::new(window))
}

fn resolve_optional_pair(
    first: Option<f64>,
    second: Option<f64>,
    default_first: f64,
    default_second: f64,
) -> Option<(f64, f64)> {
    match (first, second) {
        (None, None) => None,
        (first, second) => Some((
            first.unwrap_or(default_first),
            second.unwrap_or(default_second),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_optional_pair;

    #[test]
    fn test_resolve_optional_pair_returns_none_when_both_missing() {
        assert_eq!(resolve_optional_pair(None, None, 10.0, 20.0), None);
    }

    #[test]
    fn test_resolve_optional_pair_fills_missing_first_value() {
        assert_eq!(
            resolve_optional_pair(None, Some(20.0), 10.0, 99.0),
            Some((10.0, 20.0))
        );
    }

    #[test]
    fn test_resolve_optional_pair_fills_missing_second_value() {
        assert_eq!(
            resolve_optional_pair(Some(10.0), None, 99.0, 20.0),
            Some((10.0, 20.0))
        );
    }
}
