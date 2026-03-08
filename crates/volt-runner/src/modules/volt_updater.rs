#[path = "volt_updater/api.rs"]
mod api;
#[path = "volt_updater/config.rs"]
mod config;
#[path = "volt_updater/events.rs"]
mod events;
#[path = "volt_updater/operations.rs"]
mod operations;
#[path = "volt_updater/serialization.rs"]
mod serialization;
#[path = "volt_updater/state.rs"]
mod state;

pub use api::build_module;

pub(crate) use state::{prepare_startup_recovery, spawn_healthy_startup_clearer};

#[cfg(test)]
#[path = "volt_updater/tests.rs"]
mod tests;
