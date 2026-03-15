use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory, FakeRequestOutcome};
use super::shared::manager_with_factory;
use crate::plugin_manager::process::WireMessage;
use crate::runner::config::RunnerPluginConfig;

mod commands;
mod events;
mod fs;

fn manager_for_registration_tests(
    requests_seen: Arc<Mutex<Vec<(String, Value)>>>,
    sent_events: Arc<Mutex<Vec<(String, Value)>>>,
) -> (PluginManager, Arc<FakeProcessFactory>) {
    let root = TempDir::new("registrations");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let factory = Arc::new(FakeProcessFactory::new(HashMap::from([(
        "acme.search".to_string(),
        FakePlan {
            requests: HashMap::from([
                (
                    "plugin:invoke-command".to_string(),
                    FakeRequestOutcome::Success(json!({ "ok": true })),
                ),
                (
                    "plugin:invoke-ipc".to_string(),
                    FakeRequestOutcome::Success(json!({ "result": "pong" })),
                ),
            ]),
            requests_seen,
            sent_events,
            ..FakePlan::default()
        },
    )])));
    let manager = manager_with_factory(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        factory.clone(),
    );
    (manager, factory)
}

fn request_message(id: &str, method: &str, payload: Value) -> WireMessage {
    WireMessage {
        message_type: WireMessageType::Request,
        id: id.to_string(),
        method: method.to_string(),
        payload: Some(payload),
        error: None,
    }
}
