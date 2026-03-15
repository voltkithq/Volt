use crate::config::{DelegatedGrant, PluginConfig};
use crate::runtime_state::configure_mock;

mod activation;
mod grants;

fn build_config(script_name: &str, source: &str) -> PluginConfig {
    let temp_dir = std::env::temp_dir().join(format!(
        "volt-plugin-host-{script_name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_dir).expect("temp dir");
    let script_path = temp_dir.join(format!("{script_name}.mjs"));
    std::fs::write(&script_path, source).expect("plugin script");

    PluginConfig {
        plugin_id: "acme.search".into(),
        backend_entry: script_path.display().to_string(),
        manifest: serde_json::json!({ "id": "acme.search", "name": "Acme Search" }),
        capabilities: vec!["fs".into()],
        data_root: temp_dir.display().to_string(),
        delegated_grants: vec![],
        host_ipc_settings: None,
    }
}

fn build_config_with_grants(
    script_name: &str,
    source: &str,
    delegated_grants: Vec<DelegatedGrant>,
) -> PluginConfig {
    let mut config = build_config(script_name, source);
    config.delegated_grants = delegated_grants;
    config
}

fn configure_engine(config: &PluginConfig, inbound: Vec<crate::ipc::IpcMessage>) {
    configure_mock(config, inbound);
}
