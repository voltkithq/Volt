use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::FakeProcessFactory;
use super::shared::manager_with_factory;
use crate::runner::config::RunnerPluginConfig;

#[test]
fn discovery_finds_manifests_and_reports_missing_directories() {
    let root = TempDir::new("discovery");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![
                root.join("plugins").display().to_string(),
                root.join("missing").display().to_string(),
            ],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .state,
        PluginState::Validated
    );
    assert_eq!(manager.discovery_issues().len(), 1);
}

#[test]
fn discovery_reports_invalid_manifest_without_registering_plugin() {
    let root = TempDir::new("invalid-manifest");
    let manifest_path = root.join("plugins/acme.broken/volt-plugin.json");
    std::fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
        .expect("manifest dir");
    std::fs::write(
        &manifest_path,
        br#"{
            "id": "acme.broken",
            "name": "Broken Plugin",
            "version": "0.1.0",
            "apiVersion": 1,
            "engine": { "volt": "^0.1.0" },
            "backend": "./dist/missing.js",
            "capabilities": ["fs"]
        }"#,
    )
    .expect("manifest");
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.broken".to_string()],
            grants: BTreeMap::from([("acme.broken".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    assert!(manager.get_plugin_state("acme.broken").is_none());
    let issues = manager.discovery_issues();
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].path.as_ref(), Some(&manifest_path));
    assert!(issues[0].message.contains("does not exist"));
    assert!(issues[1].message.contains("enabled plugin 'acme.broken'"));
}

#[test]
fn capability_intersection_rejects_unsatisfied_plugins_and_keeps_exact_matches() {
    let root = TempDir::new("capabilities");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs", "http"],
    );
    write_manifest(
        &root.join("plugins/acme.clip/volt-plugin.json"),
        "acme.clip",
        &["fs"],
    );
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string(), "acme.clip".to_string()],
            grants: BTreeMap::from([
                ("acme.search".to_string(), vec!["fs".to_string()]),
                (
                    "acme.clip".to_string(),
                    vec!["fs".to_string(), "http".to_string()],
                ),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
    );

    let failed = manager.get_plugin_state("acme.search").expect("search");
    assert_eq!(failed.state, PluginState::Failed);
    assert_eq!(failed.plugin_id, "acme.search");
    assert!(failed.enabled);
    assert!(
        failed
            .manifest_path
            .ends_with(Path::new("acme.search").join("volt-plugin.json"))
    );
    assert_eq!(
        failed.requested_capabilities,
        vec!["fs".to_string(), "http".to_string()]
    );
    assert_eq!(failed.effective_capabilities, vec!["fs".to_string()]);
    assert!(failed.data_root.as_ref().expect("data root").exists());
    assert_eq!(failed.transitions.len(), 2);
    assert_eq!(failed.transitions[0].new_state, PluginState::Discovered);
    assert_eq!(failed.transitions[1].new_state, PluginState::Failed);
    assert_eq!(failed.errors.len(), 1);
    assert_eq!(failed.errors[0].plugin_id, "acme.search");
    assert_eq!(failed.errors[0].state, PluginState::Failed);
    assert_eq!(failed.errors[0].code, PLUGIN_NOT_AVAILABLE_CODE);
    assert!(failed.errors[0].message.contains("unsatisfiable"));
    assert!(failed.errors[0].details.is_some());
    assert!(failed.errors[0].stderr.is_none());
    assert!(failed.errors[0].timestamp_ms > 0);
    assert_eq!(failed.metrics.pid, None);
    assert_eq!(failed.metrics.missed_heartbeats, 0);
    assert!(!failed.process_running);

    let exact = manager.get_plugin_state("acme.clip").expect("clip");
    assert_eq!(exact.state, PluginState::Validated);
    assert_eq!(exact.effective_capabilities, vec!["fs".to_string()]);
}

#[test]
fn boot_rule_validation_does_not_spawn_plugin_processes() {
    let root = TempDir::new("boot");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let factory = Arc::new(FakeProcessFactory::new(HashMap::new()));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );

    assert_eq!(
        factory
            .spawn_count
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .state,
        PluginState::Validated
    );
    assert!(
        manager
            .get_plugin_state("acme.search")
            .expect("plugin")
            .data_root
            .expect("data root")
            .exists()
    );
}
