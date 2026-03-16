use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::{
    PluginError, PluginLifecycleEvent, PluginRecord, PluginRegistrationSnapshot, PluginState,
    PluginStateSnapshot, PluginStateTransition,
};

#[derive(Debug, Clone)]
pub(super) struct PluginLifecycle {
    state: Option<PluginState>,
    pub(super) transitions: Vec<PluginStateTransition>,
    pub(super) errors: Vec<PluginError>,
    consecutive_failures: u32,
}

impl PluginRecord {
    pub(super) fn snapshot(&self) -> PluginStateSnapshot {
        PluginStateSnapshot {
            plugin_id: self.manifest.id.clone(),
            current_state: self.lifecycle.current_state(),
            enabled: self.enabled,
            manifest_path: self.manifest_path.clone(),
            data_root: self.data_root.clone(),
            requested_capabilities: self.requested_capabilities.iter().cloned().collect(),
            effective_capabilities: self.effective_capabilities.iter().cloned().collect(),
            transition_history: self.lifecycle.transitions.clone(),
            errors: self.lifecycle.errors.clone(),
            metrics: self.metrics.clone(),
            process_running: self.process.is_some(),
            active_registrations: PluginRegistrationSnapshot {
                command_count: self.registrations.commands.len(),
                event_subscription_count: self.registrations.event_subscriptions.len(),
                ipc_handler_count: self.registrations.ipc_handlers.len(),
            },
            delegated_grant_count: self.delegated_grants.len(),
            consecutive_failures: self.lifecycle.consecutive_failures(),
        }
    }
}

impl PluginLifecycle {
    pub(super) fn new() -> Self {
        Self {
            state: None,
            transitions: Vec::new(),
            errors: Vec::new(),
            consecutive_failures: 0,
        }
    }

    pub(super) fn transition(
        &mut self,
        next_state: PluginState,
    ) -> Result<PluginStateTransition, String> {
        if let Some(current_state) = self.state
            && !is_valid_transition(current_state, next_state)
        {
            return Err(format!(
                "invalid plugin state transition: {:?} -> {:?}",
                current_state, next_state
            ));
        }
        if self.state.is_none() && next_state != PluginState::Discovered {
            return Err(format!(
                "invalid initial plugin state transition: {:?}",
                next_state
            ));
        }

        let transition = PluginStateTransition {
            previous_state: self.state,
            new_state: next_state,
            timestamp_ms: now_ms(),
        };
        self.state = Some(next_state);
        self.transitions.push(transition.clone());
        if next_state == PluginState::Running {
            self.consecutive_failures = 0;
        }
        Ok(transition)
    }

    pub(super) fn fail(
        &mut self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
        max_errors: usize,
    ) -> Result<(PluginStateTransition, PluginError, u32), String> {
        let failure_state = self.state.unwrap_or(PluginState::Failed);
        let transition = if self.state.is_none() {
            let transition = PluginStateTransition {
                previous_state: None,
                new_state: PluginState::Failed,
                timestamp_ms: now_ms(),
            };
            self.state = Some(PluginState::Failed);
            self.transitions.push(transition.clone());
            transition
        } else {
            self.transition(PluginState::Failed)?
        };
        let error = self.push_error(
            PluginError {
                plugin_id: plugin_id.to_string(),
                state: failure_state,
                code: code.to_string(),
                message,
                details,
                stderr,
                timestamp_ms: transition.timestamp_ms,
            },
            max_errors,
        );
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        Ok((transition, error, self.consecutive_failures))
    }

    pub(super) fn push_error(&mut self, error: PluginError, max_errors: usize) -> PluginError {
        self.errors.push(error.clone());
        trim_error_history(&mut self.errors, max_errors);
        error
    }

    pub(super) fn current_state(&self) -> PluginState {
        self.state.expect("plugin lifecycle must be initialized")
    }

    pub(super) fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    pub(super) fn reset_failures(&mut self) {
        self.consecutive_failures = 0;
    }

    pub(super) fn recorded_events(&self, plugin_id: &str) -> Vec<PluginLifecycleEvent> {
        self.transitions
            .iter()
            .map(|transition| PluginLifecycleEvent {
                plugin_id: plugin_id.to_string(),
                previous_state: transition.previous_state,
                new_state: transition.new_state,
                timestamp: transition.timestamp_ms,
                error: self
                    .errors
                    .iter()
                    .find(|error| error.timestamp_ms == transition.timestamp_ms)
                    .cloned(),
            })
            .collect()
    }
}

fn trim_error_history(errors: &mut Vec<PluginError>, max_errors: usize) {
    if errors.len() > max_errors {
        let drop_count = errors.len() - max_errors;
        errors.drain(0..drop_count);
    }
}

fn is_valid_transition(current: PluginState, next: PluginState) -> bool {
    if next == PluginState::Failed || next == PluginState::Disabled {
        return true;
    }
    if current == next {
        return current == PluginState::Terminated;
    }

    matches!(
        (current, next),
        (PluginState::Discovered, PluginState::Validated)
            | (PluginState::Validated, PluginState::Spawning)
            | (PluginState::Terminated, PluginState::Spawning)
            | (PluginState::Failed, PluginState::Spawning)
            | (PluginState::Disabled, PluginState::Validated)
            | (PluginState::Spawning, PluginState::Terminated)
            | (PluginState::Spawning, PluginState::Loaded)
            | (PluginState::Loaded, PluginState::Terminated)
            | (PluginState::Loaded, PluginState::Active)
            | (PluginState::Active, PluginState::Running)
            | (PluginState::Active, PluginState::Deactivating)
            | (PluginState::Running, PluginState::Deactivating)
            | (PluginState::Deactivating, PluginState::Terminated)
    )
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis() as u64
}
