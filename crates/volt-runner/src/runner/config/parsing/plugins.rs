use serde_json::Value;
use volt_core::permissions::Permission;

use super::super::{
    RunnerError, RunnerPluginConfig, RunnerPluginLimits, RunnerPluginSpawning,
    RunnerPluginSpawningStrategy,
};
use super::scalars::{parse_positive_u64, parse_string_array};

pub(super) fn parse_plugin_config(
    value: Option<&Value>,
) -> Result<RunnerPluginConfig, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginConfig::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins must be an object when provided".to_string(),
        ));
    };

    let enabled = parse_string_array(object.get("enabled"), "plugins.enabled")?;
    let grants = parse_plugin_grants(object.get("grants"))?;
    let plugin_dirs = parse_string_array(object.get("pluginDirs"), "plugins.pluginDirs")?;
    let limits = parse_plugin_limits(object.get("limits"))?;
    let spawning = parse_plugin_spawning(object.get("spawning"))?;

    Ok(RunnerPluginConfig {
        enabled,
        grants,
        plugin_dirs,
        limits,
        spawning,
    })
}

fn parse_plugin_grants(
    value: Option<&Value>,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, RunnerError> {
    let Some(value) = value else {
        return Ok(std::collections::BTreeMap::new());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.grants must be an object when provided".to_string(),
        ));
    };

    let mut grants = std::collections::BTreeMap::new();
    for (plugin_id, granted) in object {
        if plugin_id.trim().is_empty() {
            return Err(RunnerError::Config(
                "plugins.grants keys must not be empty".to_string(),
            ));
        }
        let Some(values) = granted.as_array() else {
            return Err(RunnerError::Config(format!(
                "plugins.grants.{plugin_id} must be an array"
            )));
        };

        let mut parsed = Vec::with_capacity(values.len());
        for (index, entry) in values.iter().enumerate() {
            let Some(name) = entry.as_str() else {
                return Err(RunnerError::Config(format!(
                    "plugins.grants.{plugin_id}[{index}] must be a string"
                )));
            };
            if Permission::from_str_name(name).is_none() {
                return Err(RunnerError::Config(format!(
                    "plugins.grants.{plugin_id}[{index}] contains unknown permission '{name}'"
                )));
            }
            parsed.push(name.to_string());
        }
        grants.insert(plugin_id.to_string(), parsed);
    }

    Ok(grants)
}

fn parse_plugin_limits(value: Option<&Value>) -> Result<RunnerPluginLimits, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginLimits::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.limits must be an object when provided".to_string(),
        ));
    };

    Ok(RunnerPluginLimits {
        activation_timeout_ms: parse_positive_u64(
            object.get("activationTimeoutMs"),
            "plugins.limits.activationTimeoutMs",
            RunnerPluginLimits::default().activation_timeout_ms,
        )?,
        deactivation_timeout_ms: parse_positive_u64(
            object.get("deactivationTimeoutMs"),
            "plugins.limits.deactivationTimeoutMs",
            RunnerPluginLimits::default().deactivation_timeout_ms,
        )?,
        call_timeout_ms: parse_positive_u64(
            object.get("callTimeoutMs"),
            "plugins.limits.callTimeoutMs",
            RunnerPluginLimits::default().call_timeout_ms,
        )?,
        max_plugins: usize::try_from(parse_positive_u64(
            object.get("maxPlugins"),
            "plugins.limits.maxPlugins",
            RunnerPluginLimits::default().max_plugins as u64,
        )?)
        .map_err(|_| {
            RunnerError::Config("plugins.limits.maxPlugins is too large for this platform".into())
        })?,
        heartbeat_interval_ms: parse_positive_u64(
            object.get("heartbeatIntervalMs"),
            "plugins.limits.heartbeatIntervalMs",
            RunnerPluginLimits::default().heartbeat_interval_ms,
        )?,
        heartbeat_timeout_ms: parse_positive_u64(
            object.get("heartbeatTimeoutMs"),
            "plugins.limits.heartbeatTimeoutMs",
            RunnerPluginLimits::default().heartbeat_timeout_ms,
        )?,
    })
}

fn parse_plugin_spawning(value: Option<&Value>) -> Result<RunnerPluginSpawning, RunnerError> {
    let Some(value) = value else {
        return Ok(RunnerPluginSpawning::default());
    };
    let Some(object) = value.as_object() else {
        return Err(RunnerError::Config(
            "plugins.spawning must be an object when provided".to_string(),
        ));
    };

    let strategy = match object.get("strategy").and_then(Value::as_str) {
        None => RunnerPluginSpawningStrategy::Lazy,
        Some("lazy") => RunnerPluginSpawningStrategy::Lazy,
        Some("eager") => RunnerPluginSpawningStrategy::Eager,
        Some(other) => {
            return Err(RunnerError::Config(format!(
                "plugins.spawning.strategy must be 'lazy' or 'eager', got '{other}'"
            )));
        }
    };

    Ok(RunnerPluginSpawning {
        strategy,
        idle_timeout_ms: parse_positive_u64(
            object.get("idleTimeoutMs"),
            "plugins.spawning.idleTimeoutMs",
            RunnerPluginSpawning::default().idle_timeout_ms,
        )?,
        pre_spawn: parse_string_array(object.get("preSpawn"), "plugins.spawning.preSpawn")?,
    })
}
