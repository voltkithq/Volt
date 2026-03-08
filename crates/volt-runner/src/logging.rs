use std::sync::OnceLock;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

static LOGGING_INIT: OnceLock<()> = OnceLock::new();

fn resolve_log_filter(
    default_level: &str,
    mut read_env: impl FnMut(&str) -> Option<String>,
) -> String {
    read_env("VOLT_LOG")
        .or_else(|| read_env("RUST_LOG"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_level.to_string())
}

pub(crate) fn init_logging(default_level: &'static str) {
    LOGGING_INIT.get_or_init(|| {
        let requested_filter = resolve_log_filter(default_level, |key| std::env::var(key).ok());
        let env_filter = EnvFilter::try_new(requested_filter.clone())
            .unwrap_or_else(|_| EnvFilter::new(default_level));
        let _ = tracing_log::LogTracer::init();
        let subscriber = fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .with_writer(std::io::stderr)
            .finish();
        if tracing::subscriber::set_global_default(subscriber).is_err() {
            return;
        }
        tracing::debug!(
            default_level = default_level,
            selected_filter = requested_filter,
            "logging initialized"
        );
    });
}

#[cfg(test)]
mod tests {
    use super::resolve_log_filter;

    #[test]
    fn prefers_volt_log_over_rust_log() {
        let value = resolve_log_filter("warn", |key| match key {
            "VOLT_LOG" => Some("debug,volt=trace".to_string()),
            "RUST_LOG" => Some("error".to_string()),
            _ => None,
        });
        assert_eq!(value, "debug,volt=trace");
    }

    #[test]
    fn falls_back_to_rust_log_then_default() {
        let rust_value = resolve_log_filter("warn", |key| match key {
            "RUST_LOG" => Some("info".to_string()),
            _ => None,
        });
        assert_eq!(rust_value, "info");

        let default_value = resolve_log_filter("warn", |_| None);
        assert_eq!(default_value, "warn");
    }
}
