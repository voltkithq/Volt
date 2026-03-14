use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::json;
use volt_core::permissions::Permission;

use super::{
    PLUGIN_NOT_AVAILABLE_CODE, PluginDiscoveryIssue, PluginLifecycle, PluginManager,
    PluginProcessFactory, PluginRecord, PluginRegistry, PluginResourceMetrics, PluginState,
    collect_manifest_paths, compute_effective_capabilities, ensure_plugin_data_root,
    parse_plugin_manifest, resolve_app_data_root, resolve_plugin_directory,
};
use crate::runner::config::RunnerPluginConfig;

impl PluginManager {
    pub(crate) fn new(
        app_name: String,
        permissions: &[String],
        config: RunnerPluginConfig,
    ) -> Result<Self, String> {
        Self::with_factory(
            app_name,
            permissions,
            config,
            Arc::new(super::RealPluginProcessFactory),
        )
    }

    pub(super) fn with_factory(
        app_name: String,
        permissions: &[String],
        config: RunnerPluginConfig,
        factory: Arc<dyn PluginProcessFactory>,
    ) -> Result<Self, String> {
        let app_permissions = permissions
            .iter()
            .filter_map(|name| Permission::from_str_name(name))
            .collect::<HashSet<_>>();
        let app_data_root = resolve_app_data_root(&app_name)?;
        let manager = Self {
            inner: Arc::new(super::PluginManagerInner {
                config,
                app_permissions,
                app_data_root,
                factory,
                registry: Mutex::new(PluginRegistry::default()),
            }),
        };
        manager.discover_plugins();
        Ok(manager)
    }

    pub(super) fn discover_plugins(&self) {
        let enabled = self
            .inner
            .config
            .enabled
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut manifest_paths = Vec::new();
        let mut registry = PluginRegistry::default();

        for directory in &self.inner.config.plugin_dirs {
            let resolved = resolve_plugin_directory(directory);
            if !resolved.exists() {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: Some(resolved),
                    message: format!("plugin directory '{directory}' does not exist"),
                });
                continue;
            }
            if let Err(error) = collect_manifest_paths(&resolved, &mut manifest_paths) {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: Some(resolved),
                    message: format!("failed to scan plugin directory: {error}"),
                });
            }
        }

        manifest_paths.sort();
        let mut discovered_ids = HashSet::new();
        let mut enabled_count = 0_usize;
        for manifest_path in manifest_paths {
            match self.discover_plugin_record(&manifest_path, &enabled) {
                Ok(record) => {
                    if !discovered_ids.insert(record.manifest.id.clone()) {
                        registry.discovery_issues.push(PluginDiscoveryIssue {
                            path: Some(manifest_path),
                            message: format!("duplicate plugin id '{}'", record.manifest.id),
                        });
                        continue;
                    }
                    if record.enabled {
                        enabled_count += 1;
                        if enabled_count > self.inner.config.limits.max_plugins {
                            registry.discovery_issues.push(PluginDiscoveryIssue {
                                path: Some(record.manifest_path.clone()),
                                message: format!(
                                    "max enabled plugins exceeded (limit={})",
                                    self.inner.config.limits.max_plugins
                                ),
                            });
                            continue;
                        }
                    }
                    registry.plugins.insert(record.manifest.id.clone(), record);
                }
                Err(issue) => registry.discovery_issues.push(issue),
            }
        }

        for plugin_id in enabled {
            if !discovered_ids.contains(&plugin_id) {
                registry.discovery_issues.push(PluginDiscoveryIssue {
                    path: None,
                    message: format!(
                        "enabled plugin '{plugin_id}' was not found in configured plugin directories"
                    ),
                });
            }
        }

        if let Ok(mut guard) = self.inner.registry.lock() {
            *guard = registry;
        }
    }

    fn discover_plugin_record(
        &self,
        manifest_path: &Path,
        enabled_plugins: &HashSet<String>,
    ) -> Result<PluginRecord, PluginDiscoveryIssue> {
        let plugin_root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| PluginDiscoveryIssue {
                path: Some(manifest_path.to_path_buf()),
                message: "manifest file is missing a parent directory".to_string(),
            })?;
        let manifest_bytes = fs::read(manifest_path).map_err(|error| PluginDiscoveryIssue {
            path: Some(manifest_path.to_path_buf()),
            message: format!("failed to read manifest: {error}"),
        })?;
        let manifest = parse_plugin_manifest(&manifest_bytes, &plugin_root).map_err(|message| {
            PluginDiscoveryIssue {
                path: Some(manifest_path.to_path_buf()),
                message,
            }
        })?;
        let enabled = enabled_plugins.contains(&manifest.id);
        let requested_capabilities = manifest
            .capabilities
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let effective_capabilities = compute_effective_capabilities(
            &manifest,
            &self.inner.config,
            &self.inner.app_permissions,
        );
        let data_root = if enabled {
            Some(ensure_plugin_data_root(
                &self.inner.app_data_root,
                &manifest.id,
            )?)
        } else {
            None
        };

        let mut lifecycle = PluginLifecycle::new();
        lifecycle
            .transition(PluginState::Discovered)
            .expect("discover");
        if !enabled {
            lifecycle
                .transition(PluginState::Disabled)
                .expect("disable");
        } else if requested_capabilities != effective_capabilities {
            let missing = requested_capabilities
                .difference(&effective_capabilities)
                .cloned()
                .collect::<Vec<_>>();
            lifecycle.fail(
                &manifest.id,
                PLUGIN_NOT_AVAILABLE_CODE,
                format!(
                    "requested capabilities are unsatisfiable: {}",
                    missing.join(", ")
                ),
                Some(json!({ "missingCapabilities": missing })),
                None,
            );
        } else {
            lifecycle
                .transition(PluginState::Validated)
                .expect("validate");
        }

        Ok(PluginRecord {
            manifest,
            manifest_path: manifest_path.to_path_buf(),
            enabled,
            data_root,
            requested_capabilities,
            effective_capabilities,
            lifecycle,
            metrics: PluginResourceMetrics::default(),
            process: None,
            pending_requests: 0,
            spawn_lock: Arc::new(Mutex::new(())),
        })
    }
}
