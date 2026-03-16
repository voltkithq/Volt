use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use super::{PluginError, PluginState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SubscriptionId(pub(crate) u64);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginLifecycleEvent {
    pub(crate) plugin_id: String,
    pub(crate) previous_state: Option<PluginState>,
    pub(crate) new_state: PluginState,
    pub(crate) timestamp: u64,
    pub(crate) error: Option<PluginError>,
}

#[derive(Clone, Copy)]
enum LifecycleTopic {
    All,
    Failed,
    Activated,
}

type LifecycleHandler = Arc<dyn Fn(&PluginLifecycleEvent) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct LifecycleBus {
    next_id: Arc<AtomicU64>,
    subscribers: Arc<Mutex<HashMap<SubscriptionId, (LifecycleTopic, LifecycleHandler)>>>,
}

impl LifecycleBus {
    pub(crate) fn new() -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn on_lifecycle(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.subscribe(LifecycleTopic::All, handler)
    }

    pub(crate) fn on_plugin_failed(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.subscribe(LifecycleTopic::Failed, handler)
    }

    pub(crate) fn on_plugin_activated(
        &self,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        self.subscribe(LifecycleTopic::Activated, handler)
    }

    #[allow(dead_code)]
    pub(crate) fn off(&self, subscription_id: SubscriptionId) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.remove(&subscription_id);
        }
    }

    pub(crate) fn emit(&self, event: PluginLifecycleEvent) {
        let handlers = self
            .subscribers
            .lock()
            .map(|subscribers| {
                subscribers
                    .values()
                    .filter(|(topic, _)| topic_matches(*topic, &event))
                    .map(|(_, handler)| handler.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for handler in handlers {
            if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| handler(&event))) {
                tracing::error!(
                    panic = %panic_payload_message(&payload),
                    plugin_id = %event.plugin_id,
                    new_state = ?event.new_state,
                    "plugin lifecycle subscriber panicked"
                );
            }
        }
    }

    fn subscribe(
        &self,
        topic: LifecycleTopic,
        handler: Box<dyn Fn(&PluginLifecycleEvent) + Send + Sync>,
    ) -> SubscriptionId {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::Relaxed));
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.insert(id, (topic, Arc::from(handler)));
        }
        id
    }
}

fn panic_payload_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

fn topic_matches(topic: LifecycleTopic, event: &PluginLifecycleEvent) -> bool {
    match topic {
        LifecycleTopic::All => true,
        LifecycleTopic::Failed => event.new_state == PluginState::Failed,
        LifecycleTopic::Activated => {
            matches!(
                (event.previous_state, event.new_state),
                (Some(PluginState::Loaded), PluginState::Active)
            )
        }
    }
}
