mod bridge;
mod error;
mod types;

pub use bridge::{
    BridgeLifecycle, BridgeRegistration, clear_bridge, command_observability_snapshot, init_bridge,
    is_running, record_processed_command, send_command, send_query, shutdown_bridge,
};
pub use error::CommandBridgeError;
pub use types::{AppCommand, CommandEnvelope, CommandObservabilitySnapshot, TrayCommandConfig};

#[cfg(test)]
mod tests;
