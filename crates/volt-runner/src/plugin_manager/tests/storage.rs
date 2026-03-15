use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::*;
use super::access_support::FakeAccessPicker;
use super::fs_support::{TempDir, unique_app_name, write_manifest};
use super::process_support::FakeProcessFactory;
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

fn manager_for_storage_tests() -> PluginManager {
    let root = TempDir::new("plugin-storage");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    PluginManager::with_dependencies(
        unique_app_name("Volt Storage Test"),
        &["fs".to_string(), "secureStorage".to_string()],
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(Default::default())),
        Arc::new(FakeAccessPicker::default()),
    )
    .expect("manager")
}

fn storage_request(manager: &PluginManager, method: &str, payload: Value) -> Value {
    manager
        .handle_plugin_message(
            "acme.search",
            WireMessage {
                message_type: WireMessageType::Request,
                id: method.to_string(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            },
        )
        .expect("response")
        .payload
        .unwrap_or(Value::Null)
}

fn storage_error(manager: &PluginManager, method: &str, payload: Value) -> String {
    manager
        .handle_plugin_message(
            "acme.search",
            WireMessage {
                message_type: WireMessageType::Request,
                id: method.to_string(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            },
        )
        .expect("response")
        .error
        .expect("error")
        .message
}

fn storage_root(manager: &PluginManager) -> std::path::PathBuf {
    manager
        .get_plugin_state("acme.search")
        .expect("plugin")
        .data_root
        .expect("data root")
        .join("storage")
}

#[test]
fn storage_set_get_has_delete_and_keys_roundtrip() {
    let manager = manager_for_storage_tests();

    assert_eq!(
        storage_request(&manager, "plugin:storage:get", json!({ "key": "missing" })),
        Value::Null
    );
    assert_eq!(
        storage_request(&manager, "plugin:storage:has", json!({ "key": "missing" })),
        Value::Bool(false)
    );

    let _ = storage_request(
        &manager,
        "plugin:storage:set",
        json!({ "key": "alpha", "value": "one" }),
    );
    let _ = storage_request(
        &manager,
        "plugin:storage:set",
        json!({ "key": "beta", "value": "two" }),
    );

    assert_eq!(
        storage_request(&manager, "plugin:storage:get", json!({ "key": "alpha" })),
        Value::String("one".to_string())
    );
    assert_eq!(
        storage_request(&manager, "plugin:storage:has", json!({ "key": "beta" })),
        Value::Bool(true)
    );
    assert_eq!(
        storage_request(&manager, "plugin:storage:keys", json!({})),
        json!(["alpha", "beta"])
    );

    let _ = storage_request(&manager, "plugin:storage:delete", json!({ "key": "alpha" }));
    assert_eq!(
        storage_request(&manager, "plugin:storage:get", json!({ "key": "alpha" })),
        Value::Null
    );
}

#[test]
fn storage_ignores_temp_file_when_write_crashes_before_rename() {
    let manager = manager_for_storage_tests();
    let _ = storage_request(
        &manager,
        "plugin:storage:set",
        json!({ "key": "alpha", "value": "old" }),
    );

    let mut hasher = Sha256::new();
    hasher.update("alpha".as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let storage_root = storage_root(&manager);
    std::fs::create_dir_all(&storage_root).expect("storage dir");
    std::fs::write(storage_root.join(format!("{hash}.tmp")), "new").expect("temp write");

    assert_eq!(
        storage_request(&manager, "plugin:storage:get", json!({ "key": "alpha" })),
        Value::String("old".to_string())
    );
}

#[test]
fn storage_reconciles_orphan_value_files_on_first_access() {
    let manager = manager_for_storage_tests();
    let storage_root = storage_root(&manager);
    std::fs::create_dir_all(&storage_root).expect("storage dir");
    std::fs::write(storage_root.join("orphan.val"), "orphan").expect("orphan");

    let _ = storage_request(&manager, "plugin:storage:keys", json!({}));

    assert!(!storage_root.join("orphan.val").exists());
}

#[test]
fn storage_rejects_oversized_keys_values_and_traversal() {
    let manager = manager_for_storage_tests();
    let large_key = "k".repeat(257);
    let large_value = "v".repeat(1024 * 1024 + 1);

    assert!(
        storage_error(&manager, "plugin:storage:get", json!({ "key": large_key }))
            .contains("exceeds 256 bytes")
    );
    assert!(
        storage_error(
            &manager,
            "plugin:storage:set",
            json!({ "key": "alpha", "value": large_value }),
        )
        .contains("exceeds 1048576 bytes")
    );
    assert!(
        storage_error(
            &manager,
            "plugin:storage:get",
            json!({ "key": "../escape" }),
        )
        .contains("path traversal")
    );
}
