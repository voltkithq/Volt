use std::env;
use std::path::PathBuf;

#[cfg(test)]
use volt_core::webview::WebViewConfig;
#[cfg(test)]
use volt_core::window::WindowConfig;

use super::RunnerError;
use super::config::RunnerConfig;
use super::overrides::normalize_override_path;

const ENV_FS_SCOPE_DIR: &str = "VOLT_FS_SCOPE_DIR";

pub(crate) fn resolve_fs_scope_dir(config: &RunnerConfig) -> Result<PathBuf, RunnerError> {
    if let Ok(raw_path) = env::var(ENV_FS_SCOPE_DIR) {
        let normalized = normalize_override_path(ENV_FS_SCOPE_DIR, &raw_path)?;
        return Ok(PathBuf::from(normalized));
    }

    if let Some(raw_path) = &config.fs_base_dir {
        let normalized = normalize_override_path("fsBaseDir", raw_path)?;
        return Ok(PathBuf::from(normalized));
    }

    env::current_dir().map_err(|source| RunnerError::Io {
        context: "failed to determine current working directory".to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn base_runner_config() -> RunnerConfig {
        RunnerConfig {
            app_name: "Volt".to_string(),
            devtools: false,
            permissions: Vec::new(),
            fs_base_dir: None,
            runtime_pool_size: None,
            updater_telemetry_enabled: false,
            updater_telemetry_sink: None,
            window: WindowConfig::default(),
            webview: WebViewConfig::default(),
        }
    }

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        static ENV_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    struct EnvVarReset {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarReset {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var(key).ok();
            // SAFETY: tests serialize env mutations through `env_guard()`.
            unsafe { env::set_var(key, value) };
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = env::var(key).ok();
            // SAFETY: tests serialize env mutations through `env_guard()`.
            unsafe { env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvVarReset {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                // SAFETY: tests serialize env mutations through `env_guard()`.
                unsafe { env::set_var(self.key, previous) };
            } else {
                // SAFETY: tests serialize env mutations through `env_guard()`.
                unsafe { env::remove_var(self.key) };
            }
        }
    }

    #[test]
    fn resolve_fs_scope_dir_prefers_env_override() {
        let _guard = env_guard();
        let _env = EnvVarReset::set(ENV_FS_SCOPE_DIR, "\"./env-scope\"");
        let mut config = base_runner_config();
        config.fs_base_dir = Some("./config-scope".to_string());

        let resolved = resolve_fs_scope_dir(&config).expect("env fs scope");
        assert_eq!(resolved, PathBuf::from("./env-scope"));
    }

    #[test]
    fn resolve_fs_scope_dir_uses_config_when_env_absent() {
        let _guard = env_guard();
        let _env = EnvVarReset::unset(ENV_FS_SCOPE_DIR);
        let mut config = base_runner_config();
        config.fs_base_dir = Some(" ./config-scope ".to_string());

        let resolved = resolve_fs_scope_dir(&config).expect("config fs scope");
        assert_eq!(resolved, PathBuf::from("./config-scope"));
    }

    #[test]
    fn resolve_fs_scope_dir_defaults_to_current_directory() {
        let _guard = env_guard();
        let _env = EnvVarReset::unset(ENV_FS_SCOPE_DIR);
        let config = base_runner_config();

        let resolved = resolve_fs_scope_dir(&config).expect("default fs scope");
        let current = env::current_dir().expect("current dir");
        assert_eq!(resolved, current);
    }

    #[test]
    fn resolve_fs_scope_dir_rejects_empty_env_override() {
        let _guard = env_guard();
        let _env = EnvVarReset::set(ENV_FS_SCOPE_DIR, "   ");
        let config = base_runner_config();

        let error = resolve_fs_scope_dir(&config).expect_err("empty env override should fail");
        assert!(matches!(error, RunnerError::Config(_)));
    }
}
