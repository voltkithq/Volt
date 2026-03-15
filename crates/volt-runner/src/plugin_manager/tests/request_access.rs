use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde_json::json;
use volt_core::grant_store;

use super::super::*;
use super::access_support::FakeAccessPicker;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::FakeProcessFactory;
use super::shared::manager_with_picker;
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

fn manager_for_access_tests(picker: FakeAccessPicker) -> PluginManager {
    let root = TempDir::new("plugin-access");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    write_manifest(
        &root.join("plugins/beta.index/volt-plugin.json"),
        "beta.index",
        &["fs"],
    );
    let acme_manifest_path = root.join("plugins/acme.search/volt-plugin.json");
    let manifest = serde_json::json!({
        "id": "acme.search",
        "name": "Acme Search",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs"]
    });
    std::fs::write(
        &acme_manifest_path,
        serde_json::to_vec(&manifest).expect("manifest json"),
    )
    .expect("manifest");

    manager_with_picker(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string(), "beta.index".to_string()],
            grants: BTreeMap::from([
                ("acme.search".to_string(), vec!["fs".to_string()]),
                ("beta.index".to_string(), vec!["fs".to_string()]),
            ]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::new())),
        Arc::new(picker),
    )
}

fn plugin_request(
    manager: &PluginManager,
    plugin_id: &str,
    method: &str,
    payload: serde_json::Value,
) -> WireMessage {
    manager
        .handle_plugin_message(
            plugin_id,
            WireMessage {
                message_type: WireMessageType::Request,
                id: method.to_string(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            },
        )
        .expect("response")
}

#[test]
fn request_access_returns_grant_and_only_delegates_it_to_the_requesting_plugin() {
    grant_store::clear_grants();
    let selected_root = TempDir::new("selected-folder");
    let selected_path = selected_root.join("picked");
    std::fs::create_dir_all(&selected_path).expect("selected dir");
    std::fs::write(selected_path.join("child.txt"), "ok").expect("file");
    let picker = FakeAccessPicker::from_responses(vec![Ok(Some(selected_path.clone()))]);
    let seen = picker.seen.clone();
    let manager = manager_for_access_tests(picker);

    let response = plugin_request(
        &manager,
        "acme.search",
        "plugin:request-access",
        json!({ "title": "Select search directory", "directory": true, "multiple": false }),
    );
    let payload = response.payload.expect("payload");
    let grant_id = payload["grantId"].as_str().expect("grant id").to_string();
    let selected_display = selected_path.display().to_string();
    assert_eq!(payload["path"].as_str(), Some(selected_display.as_str()));

    let seen = seen.lock().expect("seen");
    assert_eq!(seen.len(), 1);
    assert!(seen[0].title.contains("Plugin 'Acme Search'"));

    let access = plugin_request(
        &manager,
        "acme.search",
        "plugin:grant-fs:exists",
        json!({ "grantId": grant_id, "path": "child.txt" }),
    );
    assert_eq!(access.payload, Some(serde_json::Value::Bool(true)));

    let denied = plugin_request(
        &manager,
        "beta.index",
        "plugin:grant-fs:exists",
        json!({ "grantId": payload["grantId"], "path": "child.txt" }),
    );
    assert!(
        denied
            .error
            .expect("error")
            .message
            .contains("not delegated")
    );
    grant_store::clear_grants();
}

#[test]
fn request_access_returns_null_when_user_cancels() {
    grant_store::clear_grants();
    let picker = FakeAccessPicker::from_responses(vec![Ok(None)]);
    let manager = manager_for_access_tests(picker);

    let response = plugin_request(
        &manager,
        "acme.search",
        "plugin:request-access",
        json!({ "title": "Select search directory", "directory": true }),
    );

    assert_eq!(response.payload, Some(serde_json::Value::Null));
    grant_store::clear_grants();
}
