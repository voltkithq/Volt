//! `volt-plugin-host` — lightweight Boa runner for plugin processes.

mod config;
mod engine;
mod ipc;
mod modules;
mod runtime_state;

use std::io;

use config::parse_args;
use volt_permissions::CapabilityGuard;

fn init_logging() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}

fn run() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    let config = parse_args(&args)?;

    tracing::info!(
        plugin_id = %config.plugin_id,
        backend_entry = %config.backend_entry,
        data_root = %config.data_root,
        capabilities = ?config.capabilities,
        "plugin host starting"
    );

    let _guard = CapabilityGuard::from_names(&config.capabilities);
    let mut engine = engine::PluginEngine::start(&config)?;
    engine.run()?;
    tracing::info!("plugin host exiting cleanly");
    Ok(())
}

fn main() {
    init_logging();
    if let Err(error) = run() {
        tracing::error!(error = %error, "volt-plugin-host exited with error");
        std::process::exit(1);
    }
}
