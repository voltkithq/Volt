use std::time::Duration;

use serde_json::{Value, json};
use volt_core::ipc::{
    IPC_HANDLER_NOT_FOUND_CODE, IPC_HANDLER_TIMEOUT_CODE, IpcRequest, IpcResponse,
};

use super::{
    DEFAULT_PRE_SPAWN_GRACE_MS, PLUGIN_IPC_HANDLER_NOT_FOUND_CODE, PLUGIN_ROUTE_INVALID_CODE,
    PluginDiscoveryIssue, PluginManager, PluginState, parse_plugin_route,
};
use crate::runner::config::RunnerPluginSpawningStrategy;

mod lifecycle;
mod observability;
mod recovery;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PluginStartupMode {
    LoadOnly,
    Activate,
}

impl PluginManager {
    pub(crate) fn discovery_issues(&self) -> Vec<PluginDiscoveryIssue> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        registry.discovery_issues.clone()
    }

    pub(crate) fn handle_ipc_request(
        &self,
        request: &IpcRequest,
        timeout: Duration,
    ) -> Option<IpcResponse> {
        match parse_plugin_route(&request.method) {
            Ok(Some(route)) => Some(self.dispatch_plugin_request(request, &route, timeout)),
            Ok(None) => None,
            Err(message) => Some(IpcResponse::error_with_code(
                request.id.clone(),
                message,
                PLUGIN_ROUTE_INVALID_CODE.to_string(),
            )),
        }
    }

    pub(crate) fn start_pre_spawn(&self) {
        self.start_pre_spawn_after(Duration::from_millis(DEFAULT_PRE_SPAWN_GRACE_MS));
    }

    pub(crate) fn start_pre_spawn_after(&self, delay: Duration) {
        let plugin_ids = self.pre_spawn_plugin_ids();
        if plugin_ids.is_empty() {
            return;
        }

        let manager = self.clone();
        let _ = std::thread::Builder::new()
            .name("volt-plugin-pre-spawn".to_string())
            .spawn(move || {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                for plugin_id in plugin_ids {
                    let _ = manager.ensure_plugin_running(&plugin_id);
                }
            });
    }

    #[cfg(test)]
    pub(super) fn run_pre_spawn_now(&self) {
        for plugin_id in self.pre_spawn_plugin_ids() {
            let _ = self.ensure_plugin_running(&plugin_id);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn prefetch_for(&self, surface: &str) {
        let plugin_ids = {
            let Ok(registry) = self.inner.registry.lock() else {
                return;
            };
            registry
                .plugins
                .values()
                .filter(|record| {
                    record.enabled
                        && record
                            .manifest
                            .prefetch_on
                            .iter()
                            .any(|candidate| candidate == surface)
                })
                .map(|record| record.manifest.id.clone())
                .collect::<Vec<_>>()
        };

        for plugin_id in plugin_ids {
            let _ = self.ensure_plugin_loaded(&plugin_id);
        }
    }

    pub(crate) fn shutdown_all(&self) {
        let plugin_ids = {
            let Ok(registry) = self.inner.registry.lock() else {
                return;
            };
            registry.plugins.keys().cloned().collect::<Vec<_>>()
        };
        for plugin_id in plugin_ids {
            self.deactivate_plugin(&plugin_id);
        }
    }

    fn dispatch_plugin_request(
        &self,
        request: &IpcRequest,
        route: &super::PluginRoute,
        timeout: Duration,
    ) -> IpcResponse {
        let registered = {
            let Ok(registry) = self.inner.registry.lock() else {
                return IpcResponse::error_with_code(
                    request.id.clone(),
                    "plugin registry is unavailable".to_string(),
                    PLUGIN_IPC_HANDLER_NOT_FOUND_CODE.to_string(),
                );
            };
            registry.ipc_handlers.get(&request.method).cloned()
        };
        let Some(registered) = registered else {
            return IpcResponse::error_with_code(
                request.id.clone(),
                format!("plugin IPC handler not found: {}", request.method),
                IPC_HANDLER_NOT_FOUND_CODE.to_string(),
            );
        };

        let process = match self.ensure_plugin_running(&route.plugin_id) {
            Ok(process) => process,
            Err(error) => {
                return IpcResponse::error_with_details(
                    request.id.clone(),
                    error.message,
                    error.code,
                    json!({ "pluginId": route.plugin_id }),
                );
            }
        };

        self.mark_request_started(&route.plugin_id);
        let response = process.request(
            "plugin:invoke-ipc",
            json!({ "channel": registered.method, "args": request.args.clone() }),
            timeout,
        );
        self.mark_request_finished(&route.plugin_id);

        match response {
            Ok(message) => {
                self.record_activity(&route.plugin_id);
                if let Some(error) = message.error {
                    IpcResponse::error_with_code(request.id.clone(), error.message, error.code)
                } else {
                    IpcResponse::success(request.id.clone(), message.payload.unwrap_or(Value::Null))
                }
            }
            Err(error) if error.code == "TIMEOUT" => IpcResponse::error_with_details(
                request.id.clone(),
                error.message,
                IPC_HANDLER_TIMEOUT_CODE.to_string(),
                json!({
                    "timeoutMs": timeout.as_millis(),
                    "method": request.method
                }),
            ),
            Err(error) => IpcResponse::error_with_details(
                request.id.clone(),
                error.message,
                error.code,
                json!({ "pluginId": route.plugin_id }),
            ),
        }
    }

    fn pre_spawn_plugin_ids(&self) -> Vec<String> {
        let Ok(registry) = self.inner.registry.lock() else {
            return Vec::new();
        };
        let mut plugin_ids = match self.inner.config.spawning.strategy {
            RunnerPluginSpawningStrategy::Lazy => self.inner.config.spawning.pre_spawn.clone(),
            RunnerPluginSpawningStrategy::Eager => registry
                .plugins
                .values()
                .filter(|record| {
                    record.enabled && record.lifecycle.current_state() == PluginState::Validated
                })
                .map(|record| record.manifest.id.clone())
                .collect::<Vec<_>>(),
        };
        plugin_ids.sort();
        plugin_ids.dedup();
        plugin_ids
    }
}
