use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use semver::{Version, VersionReq};
use serde_json::Value;
use volt_core::permissions::Permission;

use super::{PluginManifest, PluginRoute};
use crate::runner::config::RunnerPluginConfig;

const HOST_VOLT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SUPPORTED_PLUGIN_API_VERSIONS: &[u64] = &[1];

pub(super) fn parse_plugin_manifest(
    contents: &[u8],
    plugin_root: &Path,
) -> Result<PluginManifest, String> {
    let value: Value = serde_json::from_slice(contents)
        .map_err(|error| format!("manifest is not valid JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "manifest must be a JSON object".to_string())?;

    let id = required_string_field(object, "id")?;
    if !is_valid_reverse_domain(&id) {
        return Err("manifest id must be in reverse-domain format".to_string());
    }
    let _name = required_string_field(object, "name")?;
    let version = required_string_field(object, "version")?;
    Version::parse(&version)
        .map_err(|error| format!("manifest version must be valid semver: {error}"))?;

    let api_version = object
        .get("apiVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest apiVersion must be a positive integer".to_string())?;
    if !SUPPORTED_PLUGIN_API_VERSIONS.contains(&api_version) {
        return Err(format!("unsupported plugin apiVersion '{api_version}'"));
    }

    let engine = object
        .get("engine")
        .and_then(Value::as_object)
        .ok_or_else(|| "manifest engine must be an object".to_string())?;
    let engine_volt = required_string_field(engine, "volt")?;
    let version_req = VersionReq::parse(&engine_volt)
        .map_err(|error| format!("manifest engine.volt must be a valid semver range: {error}"))?;
    let host_version = Version::parse(HOST_VOLT_VERSION)
        .map_err(|error| format!("failed to parse host version: {error}"))?;
    if !version_req.matches(&host_version) {
        return Err(format!(
            "plugin requires Volt '{engine_volt}', host version is '{HOST_VOLT_VERSION}'"
        ));
    }

    let backend = required_string_field(object, "backend")?;
    if !backend.ends_with(".js") && !backend.ends_with(".mjs") {
        return Err("manifest backend must end with .js or .mjs".to_string());
    }
    let backend_path = plugin_root.join(&backend);
    if !backend_path.is_file() {
        return Err(format!(
            "plugin backend entry '{}' does not exist",
            backend_path.display()
        ));
    }

    let capabilities = object
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest capabilities must be an array".to_string())?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let capability = value
                .as_str()
                .ok_or_else(|| format!("manifest capabilities[{index}] must be a string"))?;
            if Permission::from_str_name(capability).is_none() {
                return Err(format!(
                    "manifest capabilities[{index}] contains unknown capability '{capability}'"
                ));
            }
            Ok(capability.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen = HashSet::new();
    for capability in &capabilities {
        if !seen.insert(capability.clone()) {
            return Err(format!(
                "manifest capabilities contains duplicate capability '{capability}'"
            ));
        }
    }

    if let Some(contributes) = object.get("contributes") {
        validate_contributes(contributes)?;
    }
    if let Some(signature) = object.get("signature") {
        validate_signature(signature)?;
    }

    Ok(PluginManifest { id, capabilities })
}

pub(super) fn compute_effective_capabilities(
    manifest: &PluginManifest,
    config: &RunnerPluginConfig,
    app_permissions: &HashSet<Permission>,
) -> BTreeSet<String> {
    let requested = manifest
        .capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let host_grants = config
        .grants
        .get(&manifest.id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let app_permissions = app_permissions
        .iter()
        .map(|permission| permission.as_str().to_string())
        .collect::<BTreeSet<_>>();

    requested
        .intersection(&host_grants)
        .cloned()
        .collect::<BTreeSet<_>>()
        .intersection(&app_permissions)
        .cloned()
        .collect()
}

pub(super) fn parse_plugin_route(method: &str) -> Result<Option<PluginRoute>, String> {
    if !method.starts_with("plugin:") {
        return Ok(None);
    }

    let route = method.trim_start_matches("plugin:");
    let Some((plugin_id, channel)) = route.split_once(':') else {
        return Err("plugin IPC routes must use 'plugin:<plugin-id>:<channel>'".to_string());
    };
    if plugin_id.trim().is_empty() || channel.trim().is_empty() {
        return Err("plugin IPC routes must include both plugin id and channel".to_string());
    }

    Ok(Some(PluginRoute {
        plugin_id: plugin_id.to_string(),
        method: channel.to_string(),
    }))
}

fn validate_contributes(value: &Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err("manifest contributes must be an object".to_string());
    };
    if let Some(commands) = object.get("commands") {
        let Some(commands) = commands.as_array() else {
            return Err("manifest contributes.commands must be an array".to_string());
        };
        for (index, command) in commands.iter().enumerate() {
            let Some(command) = command.as_object() else {
                return Err(format!(
                    "manifest contributes.commands[{index}] must be an object"
                ));
            };
            required_string_field(command, "id").map_err(|_| {
                format!("manifest contributes.commands[{index}].id must be a non-empty string")
            })?;
            required_string_field(command, "title").map_err(|_| {
                format!("manifest contributes.commands[{index}].title must be a non-empty string")
            })?;
        }
    }
    Ok(())
}

fn validate_signature(value: &Value) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err("manifest signature must be an object".to_string());
    };
    required_string_field(object, "algorithm")
        .map_err(|_| "manifest signature.algorithm must be a non-empty string".to_string())?;
    required_string_field(object, "value")
        .map_err(|_| "manifest signature.value must be a non-empty string".to_string())?;
    Ok(())
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, String> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("manifest {field} must be a non-empty string"))?;
    Ok(value.to_string())
}

fn is_valid_reverse_domain(id: &str) -> bool {
    // Plugin IDs deliberately restrict segments to lowercase ASCII alphanumerics.
    // Hyphens remain reserved so host-side namespacing stays predictable.
    let segments = id.split('.').collect::<Vec<_>>();
    if segments.len() < 2 {
        return false;
    }

    segments.iter().all(|segment| {
        let mut chars = segment.chars();
        match chars.next() {
            Some(first) if first.is_ascii_lowercase() => {
                chars.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
            }
            _ => false,
        }
    })
}
