use std::collections::BTreeMap;
use std::fs;

use volt_core::grant_store;

use super::super::{runtime_with_plugin_manager, unique_temp_dir};
use crate::plugin_manager::PluginManager;
use crate::runner::config::RunnerPluginConfig;

fn build_plugin_manager() -> (PluginManager, String) {
    let root = unique_temp_dir("plugins-module");
    let plugin_root = root.join("plugins").join("acme.search");
    let manifest_path = plugin_root.join("volt-plugin.json");
    let backend_path = plugin_root.join("dist").join("plugin.js");

    fs::create_dir_all(backend_path.parent().expect("backend parent")).expect("backend dir");
    fs::write(&backend_path, b"export default {};\n").expect("backend");
    fs::write(
        &manifest_path,
        serde_json::to_vec(&serde_json::json!({
            "id": "acme.search",
            "name": "Acme Search",
            "version": "0.1.0",
            "apiVersion": 1,
            "engine": { "volt": "^0.1.0" },
            "backend": "./dist/plugin.js",
            "capabilities": ["fs"],
            "prefetchOn": ["search-panel"]
        }))
        .expect("manifest json"),
    )
    .expect("manifest");

    let grant_root = root.join("selected");
    fs::create_dir_all(&grant_root).expect("grant root");
    let grant_id = grant_store::create_grant(grant_root).expect("grant");
    let manager = PluginManager::new(
        "Volt Test".to_string(),
        &["fs".to_string()],
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
    )
    .expect("plugin manager");
    (manager, grant_id)
}

#[test]
fn plugins_module_can_delegate_and_revoke_grants() {
    let (manager, grant_id) = build_plugin_manager();
    let runtime = runtime_with_plugin_manager(
        unique_temp_dir("plugins-runtime"),
        &["fs"],
        Some(manager.clone()),
    );

    runtime
        .client()
        .eval_promise_string(&format!(
            "(async () => {{
                const plugins = globalThis.__volt.plugins;
                await plugins.delegateGrant('acme.search', '{grant_id}');
                return 'delegated';
            }})()"
        ))
        .expect("delegate grant");
    assert_eq!(
        manager
            .list_delegated_grants("acme.search")
            .expect("delegated list"),
        vec![grant_id.clone()]
    );

    runtime
        .client()
        .eval_promise_string(&format!(
            "(async () => {{
                const plugins = globalThis.__volt.plugins;
                await plugins.revokeGrant('acme.search', '{grant_id}');
                return 'revoked';
            }})()"
        ))
        .expect("revoke grant");
    assert!(
        manager
            .list_delegated_grants("acme.search")
            .expect("delegated list after revoke")
            .is_empty()
    );
    assert!(grant_store::resolve_grant(&grant_id).is_ok());
}

#[test]
fn plugins_module_prefetch_for_is_available() {
    let (manager, _) = build_plugin_manager();
    let runtime =
        runtime_with_plugin_manager(unique_temp_dir("plugins-prefetch"), &["fs"], Some(manager));

    let result = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                const plugins = globalThis.__volt.plugins;
                await plugins.prefetchFor('search-panel');
                return 'ok';
            })()",
        )
        .expect("prefetch");

    assert_eq!(result, "ok");
}

#[test]
fn plugins_module_exposes_state_and_error_queries() {
    let (manager, _) = build_plugin_manager();
    manager.fail_plugin(
        "acme.search",
        "PLUGIN_BROKEN",
        "boom".to_string(),
        None,
        None,
    );
    let runtime = runtime_with_plugin_manager(
        unique_temp_dir("plugins-observability"),
        &["fs"],
        Some(manager),
    );

    let result = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                const plugins = globalThis.__volt.plugins;
                const state = await plugins.getPluginState('acme.search');
                const errors = await plugins.getPluginErrors('acme.search');
                return `${state.currentState}:${errors.length}:${errors[0].code}`;
            })()",
        )
        .expect("plugin state");

    assert_eq!(result, "failed:1:PLUGIN_BROKEN");
}

#[test]
fn plugins_module_receives_lifecycle_events_via_native_bridge() {
    let (manager, _) = build_plugin_manager();
    let runtime = runtime_with_plugin_manager(
        unique_temp_dir("plugins-lifecycle-events"),
        &["fs"],
        Some(manager.clone()),
    );
    let runtime_client = runtime.client();
    let _subscription = manager.on_lifecycle(Box::new(move |event| {
        let payload = serde_json::to_value(event).expect("serialize event");
        runtime_client
            .dispatch_native_event("plugin:lifecycle", payload)
            .expect("dispatch lifecycle event");
    }));

    runtime
        .client()
        .eval_unit(
            "(async () => {
                const plugins = globalThis.__volt.plugins;
                globalThis.__pluginLifecycleEvents = [];
                const handler = (event) => globalThis.__pluginLifecycleEvents.push(event.newState);
                globalThis.__pluginLifecycleHandler = handler;
                plugins.on('plugin:lifecycle', handler);
            })()",
        )
        .expect("bind lifecycle handler");

    manager.fail_plugin(
        "acme.search",
        "PLUGIN_BROKEN",
        "boom".to_string(),
        None,
        None,
    );

    let seen = runtime
        .client()
        .eval_string("globalThis.__pluginLifecycleEvents.join(',')")
        .expect("captured events");
    assert_eq!(seen, "failed");

    runtime
        .client()
        .eval_unit(
            "globalThis.__volt.plugins.off('plugin:lifecycle', globalThis.__pluginLifecycleHandler)",
        )
        .expect("unbind lifecycle handler");
    manager.retry_plugin("acme.search").expect("retry");

    let after_off = runtime
        .client()
        .eval_string("globalThis.__pluginLifecycleEvents.join(',')")
        .expect("events after off");
    assert_eq!(after_off, "failed");
}
