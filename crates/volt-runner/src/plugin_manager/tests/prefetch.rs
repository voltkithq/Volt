use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::super::*;
use super::fs_support::TempDir;
use super::process_support::FakeProcessFactory;
use super::shared::manager_with_factory;
use crate::runner::config::RunnerPluginConfig;

fn write_prefetch_manifest(root: &TempDir, id: &str, prefetch_on: &[&str]) {
    let manifest = serde_json::json!({
        "id": id,
        "name": id,
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs"],
        "prefetchOn": prefetch_on,
    });
    let manifest_path = root.join(&format!("plugins/{id}/volt-plugin.json"));
    std::fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
        .expect("manifest dir");
    std::fs::write(
        &manifest_path,
        serde_json::to_vec(&manifest).expect("manifest json"),
    )
    .expect("manifest");
    let backend = manifest_path
        .parent()
        .expect("plugin root")
        .join("dist/plugin.js");
    std::fs::create_dir_all(backend.parent().expect("backend parent")).expect("backend dir");
    std::fs::write(backend, b"export default {};\n").expect("backend");
}

fn prefetch_manager() -> (PluginManager, Arc<FakeProcessFactory>) {
    let root = TempDir::new("prefetch");
    write_prefetch_manifest(&root, "acme.search", &["search-panel"]);
    write_prefetch_manifest(&root, "beta.index", &["file-explorer"]);
    let factory = Arc::new(FakeProcessFactory::new(HashMap::new()));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string(), "beta.index".to_string()],
            grants: BTreeMap::from([
                ("acme.search".to_string(), vec!["fs".to_string()]),
                ("beta.index".to_string(), vec!["fs".to_string()]),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );
    (manager, factory)
}

#[test]
fn prefetch_spawns_matching_plugin_without_activation() {
    let (manager, factory) = prefetch_manager();

    manager.prefetch_for("search-panel");

    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 1);
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("search")
            .state,
        PluginState::Loaded
    );
    assert_eq!(
        manager.get_plugin_state("beta.index").expect("beta").state,
        PluginState::Validated
    );
}

#[test]
fn prefetch_does_not_respawn_already_loaded_plugin() {
    let (manager, factory) = prefetch_manager();

    manager.prefetch_for("search-panel");
    manager.prefetch_for("search-panel");

    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 1);
}

#[test]
fn prefetch_ignores_non_matching_surfaces() {
    let (manager, factory) = prefetch_manager();

    manager.prefetch_for("settings-panel");

    assert_eq!(factory.spawn_count.load(Ordering::Relaxed), 0);
    assert_eq!(
        manager
            .get_plugin_state("acme.search")
            .expect("search")
            .state,
        PluginState::Validated
    );
}
