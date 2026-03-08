use std::fs;
use std::path::PathBuf;

use volt_core::fs as volt_fs;
use volt_core::permissions::Permission;
use volt_core::security;

use crate::modules::require_permission;

pub(super) fn ensure_database_permission() -> Result<(), String> {
    require_permission(Permission::Database).map_err(|error| error.to_string())
}

fn database_root_dir() -> Result<PathBuf, String> {
    let base = if let Some(data_dir) = dirs::data_local_dir() {
        data_dir
    } else {
        std::env::current_dir().map_err(|error| {
            format!("failed to resolve current working directory for database storage: {error}")
        })?
    };

    Ok(base
        .join("volt")
        .join(sanitize_app_namespace(&crate::modules::app_name()?)))
}

fn sanitize_app_namespace(app_name: &str) -> String {
    let sanitized = app_name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let compact = sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        "app".to_string()
    } else {
        compact
    }
}

pub(super) fn resolve_database_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("database path must not be empty".to_string());
    }

    security::validate_path(trimmed)
        .map_err(|error| format!("invalid database path '{trimmed}': {error}"))?;

    let base_dir = database_root_dir()?;
    fs::create_dir_all(&base_dir).map_err(|error| {
        format!(
            "failed to create database directory '{}': {error}",
            base_dir.display()
        )
    })?;

    volt_fs::safe_resolve_for_create(&base_dir, trimmed).map_err(|error| {
        format!(
            "failed to resolve database path '{trimmed}' under '{}': {error}",
            base_dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_app_namespace_compacts_non_alphanumeric_segments() {
        assert_eq!(sanitize_app_namespace("Volt Demo"), "volt-demo");
        assert_eq!(sanitize_app_namespace(""), "app");
        assert_eq!(sanitize_app_namespace("  !!!  "), "app");
    }
}
