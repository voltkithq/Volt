use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::{PluginError, PluginRecord, PluginSnapshot, PluginState, PluginStateTransition};

#[derive(Debug, Clone)]
pub(super) struct PluginLifecycle {
    state: Option<PluginState>,
    pub(super) transitions: Vec<PluginStateTransition>,
    pub(super) errors: Vec<PluginError>,
}

impl PluginRecord {
    #[allow(dead_code)]
    pub(super) fn snapshot(&self) -> PluginSnapshot {
        PluginSnapshot {
            plugin_id: self.manifest.id.clone(),
            state: self.lifecycle.current_state(),
            enabled: self.enabled,
            manifest_path: self.manifest_path.clone(),
            data_root: self.data_root.clone(),
            requested_capabilities: self.requested_capabilities.iter().cloned().collect(),
            effective_capabilities: self.effective_capabilities.iter().cloned().collect(),
            transitions: self.lifecycle.transitions.clone(),
            errors: self.lifecycle.errors.clone(),
            metrics: self.metrics.clone(),
            process_running: self.process.is_some(),
        }
    }
}

impl PluginLifecycle {
    pub(super) fn new() -> Self {
        Self {
            state: None,
            transitions: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub(super) fn transition(&mut self, next_state: PluginState) -> Result<(), String> {
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

        let previous_state = self.state;
        self.state = Some(next_state);
        self.transitions.push(PluginStateTransition {
            previous_state,
            new_state: next_state,
            timestamp_ms: now_ms(),
        });
        Ok(())
    }

    pub(super) fn fail(
        &mut self,
        plugin_id: &str,
        code: &str,
        message: String,
        details: Option<Value>,
        stderr: Option<String>,
    ) {
        if self.state.is_none() {
            self.state = Some(PluginState::Failed);
            self.transitions.push(PluginStateTransition {
                previous_state: None,
                new_state: PluginState::Failed,
                timestamp_ms: now_ms(),
            });
        } else {
            let _ = self.transition(PluginState::Failed);
        }

        self.errors.push(PluginError {
            plugin_id: plugin_id.to_string(),
            state: PluginState::Failed,
            code: code.to_string(),
            message,
            details,
            stderr,
            timestamp_ms: now_ms(),
        });
    }

    pub(super) fn current_state(&self) -> PluginState {
        self.state.expect("plugin lifecycle must be initialized")
    }
}

fn is_valid_transition(current: PluginState, next: PluginState) -> bool {
    if current == next || next == PluginState::Failed || next == PluginState::Disabled {
        return true;
    }

    matches!(
        (current, next),
        (PluginState::Discovered, PluginState::Validated)
            | (PluginState::Validated, PluginState::Spawning)
            | (PluginState::Terminated, PluginState::Spawning)
            | (PluginState::Spawning, PluginState::Loaded)
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
