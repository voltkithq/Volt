use serde_json::Value;
use std::sync::{OnceLock, RwLock};
use volt_core::permissions::{CapabilityGuard, Permission};

static CAPABILITY_GUARD: OnceLock<RwLock<CapabilityGuard>> = OnceLock::new();
static CONFIGURED_PERMISSION_NAMES: OnceLock<Vec<String>> = OnceLock::new();

fn guard_slot() -> &'static RwLock<CapabilityGuard> {
    CAPABILITY_GUARD.get_or_init(|| RwLock::new(CapabilityGuard::from_names(&[])))
}

pub fn configure_permissions(config: &Value) -> napi::Result<()> {
    let names = canonicalize_permission_names(parse_permission_names(config));

    if let Some(existing) = CONFIGURED_PERMISSION_NAMES.get() {
        ensure_permission_configuration_compatible(Some(existing.as_slice()), &names)
            .map_err(napi::Error::from_reason)?;
    } else if CONFIGURED_PERMISSION_NAMES.set(names.clone()).is_err() {
        let existing = CONFIGURED_PERMISSION_NAMES
            .get()
            .expect("permission configuration should be initialized");
        ensure_permission_configuration_compatible(Some(existing.as_slice()), &names)
            .map_err(napi::Error::from_reason)?;
    }

    let mut guard = guard_slot()
        .write()
        .map_err(|e| napi::Error::from_reason(format!("Permission guard lock poisoned: {e}")))?;
    *guard = CapabilityGuard::from_names(&names);
    Ok(())
}

fn parse_permission_names(config: &Value) -> Vec<String> {
    config
        .get("permissions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn canonicalize_permission_names(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

fn ensure_permission_configuration_compatible(
    existing: Option<&[String]>,
    requested: &[String],
) -> Result<(), String> {
    if let Some(existing) = existing
        && existing != requested
    {
        return Err(format!(
            "permissions are process-global and already configured as [{}]; refusing to reconfigure as [{}]",
            existing.join(", "),
            requested.join(", ")
        ));
    }
    Ok(())
}

pub fn require_permission(permission: Permission) -> napi::Result<()> {
    let guard = guard_slot()
        .read()
        .map_err(|e| napi::Error::from_reason(format!("Permission guard lock poisoned: {e}")))?;

    guard.check(permission).map_err(|err| {
        let hint = permission_hint(permission);
        napi::Error::from_reason(format!(
            "Permission denied: {err}. Add '{name}' to permissions in volt.config.ts.{hint}",
            name = permission.as_str(),
        ))
    })
}

fn permission_hint(permission: Permission) -> &'static str {
    match permission {
        Permission::Dialog => "\n  Hint: Apps that open files typically need both 'dialog' and 'fs'.",
        Permission::FileSystem => "\n  Hint: For user-selected folders, also add 'dialog' to use showOpenWithGrant().",
        Permission::Database => "\n  Hint: The 'db' permission enables the volt:db SQLite module (backend only).",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_permission_names_reads_strings_only() {
        let config = json!({
            "permissions": ["clipboard", "shell", 42, true, null, "tray"]
        });

        let parsed = parse_permission_names(&config);
        assert_eq!(parsed, vec!["clipboard", "shell", "tray"]);
    }

    #[test]
    fn parse_permission_names_defaults_to_empty() {
        assert!(parse_permission_names(&json!({})).is_empty());
        assert!(parse_permission_names(&json!({"permissions": "not-an-array"})).is_empty());
    }

    #[test]
    fn canonicalize_permission_names_sorts_and_deduplicates() {
        let canonical = canonicalize_permission_names(vec![
            "shell".to_string(),
            "fs".to_string(),
            "shell".to_string(),
        ]);
        assert_eq!(canonical, vec!["fs".to_string(), "shell".to_string()]);
    }

    #[test]
    fn ensure_permission_configuration_compatible_rejects_reconfiguration() {
        let existing = vec!["fs".to_string()];
        let requested = vec!["shell".to_string()];
        let result = ensure_permission_configuration_compatible(Some(&existing), &requested);
        assert!(result.is_err());

        let same = ensure_permission_configuration_compatible(Some(&existing), &existing);
        assert!(same.is_ok());
    }
}
