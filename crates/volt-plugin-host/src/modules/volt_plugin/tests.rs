use super::VOLT_PLUGIN_BOOTSTRAP;

#[test]
fn bootstrap_contains_storage_and_grants_surface() {
    assert!(VOLT_PLUGIN_BOOTSTRAP.contains("storage:"));
    assert!(VOLT_PLUGIN_BOOTSTRAP.contains("requestAccess"));
    assert!(VOLT_PLUGIN_BOOTSTRAP.contains("list()"));
    assert!(VOLT_PLUGIN_BOOTSTRAP.contains("bindFsScope"));
    assert!(VOLT_PLUGIN_BOOTSTRAP.contains("__volt_plugin_revoke_grant__"));
}
