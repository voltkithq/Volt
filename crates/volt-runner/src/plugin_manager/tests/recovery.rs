use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use super::super::*;
use super::fs_support::{TempDir, write_manifest};
use super::process_support::{FakePlan, FakeProcessFactory};
use super::shared::manager_with_error_history_limit;
use crate::runner::config::RunnerPluginConfig;

fn build_manager(error_history_limit: usize) -> PluginManager {
    let root = TempDir::new("recovery");
    write_manifest(
        &root.join("plugins/acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    manager_with_error_history_limit(
        RunnerPluginConfig {
            enabled: vec!["acme.search".to_string()],
            grants: BTreeMap::from([("acme.search".to_string(), vec!["fs".to_string()])]),
            plugin_dirs: vec![root.join("plugins").display().to_string()],
            ..RunnerPluginConfig::default()
        },
        Arc::new(FakeProcessFactory::new(HashMap::from([(
            "acme.search".to_string(),
            FakePlan::default(),
        )]))),
        error_history_limit,
    )
}

#[test]
fn retry_plugin_reactivates_failed_plugin() {
    let manager = build_manager(50);

    manager.fail_plugin(
        "acme.search",
        "PLUGIN_BROKEN",
        "boom".to_string(),
        None,
        None,
    );
    manager.retry_plugin("acme.search").expect("retry");

    let snapshot = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(snapshot.current_state, PluginState::Running);
    assert_eq!(snapshot.consecutive_failures, 0);
}

#[test]
fn error_history_respects_cap_and_returns_descending_timestamps() {
    let manager = build_manager(2);

    manager.fail_plugin("acme.search", "E1", "boom-1".to_string(), None, None);
    std::thread::sleep(Duration::from_millis(2));
    manager.fail_plugin("acme.search", "E2", "boom-2".to_string(), None, None);
    std::thread::sleep(Duration::from_millis(2));
    manager.fail_plugin("acme.search", "E3", "boom-3".to_string(), None, None);

    let errors = manager.get_plugin_errors("acme.search");
    assert_eq!(errors.len(), 2);
    assert!(errors[0].timestamp_ms >= errors[1].timestamp_ms);
    assert!(errors.iter().all(|error| error.code != "E1"));
    assert_eq!(manager.get_errors().len(), 2);
}

#[test]
fn three_consecutive_failures_auto_disable_and_enable_resets_streak() {
    let manager = build_manager(50);

    for attempt in 1..=3 {
        manager.fail_plugin(
            "acme.search",
            "PLUGIN_BROKEN",
            format!("boom-{attempt}"),
            None,
            None,
        );
    }

    let disabled = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(disabled.current_state, PluginState::Disabled);
    assert_eq!(disabled.consecutive_failures, 3);
    assert!(
        disabled
            .errors
            .iter()
            .any(|error| error.code == PLUGIN_AUTO_DISABLED_CODE)
    );

    manager.enable_plugin("acme.search").expect("enable");

    let enabled = manager.get_plugin_state("acme.search").expect("plugin");
    assert_eq!(enabled.current_state, PluginState::Validated);
    assert_eq!(enabled.consecutive_failures, 0);
}
