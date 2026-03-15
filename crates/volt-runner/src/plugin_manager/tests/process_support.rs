use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use super::super::*;

#[derive(Clone)]
pub(super) struct FakeProcessFactory {
    pub(super) plans: Arc<Mutex<HashMap<String, FakePlan>>>,
    pub(super) spawn_count: Arc<AtomicU64>,
}

#[derive(Clone, Default)]
pub(super) struct FakePlan {
    pub(super) ready: FakeOutcome,
    pub(super) activate: FakeOutcome,
    pub(super) heartbeats: Vec<FakeOutcome>,
    pub(super) requests: HashMap<String, FakeRequestOutcome>,
    pub(super) requests_seen: Arc<Mutex<Vec<(String, Value)>>>,
    pub(super) sent_events: Arc<Mutex<Vec<(String, Value)>>>,
    pub(super) deactivate: FakeOutcome,
    pub(super) killed: Arc<AtomicBool>,
}

#[derive(Clone, Default)]
pub(super) enum FakeOutcome {
    #[default]
    Ok,
    Timeout,
    Error(&'static str),
    Crash(i32),
}

#[derive(Clone)]
pub(super) enum FakeRequestOutcome {
    Success(Value),
    Error(&'static str, &'static str),
    Timeout,
    Crash(i32),
}

impl FakeProcessFactory {
    pub(super) fn new(plans: HashMap<String, FakePlan>) -> Self {
        Self {
            plans: Arc::new(Mutex::new(plans)),
            spawn_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl PluginProcessFactory for FakeProcessFactory {
    fn spawn(
        &self,
        config: &PluginBootstrapConfig,
    ) -> Result<Arc<dyn PluginProcessHandle>, PluginRuntimeError> {
        self.spawn_count.fetch_add(1, Ordering::Relaxed);
        let plan = self
            .plans
            .lock()
            .expect("plans")
            .get(&config.plugin_id)
            .cloned()
            .unwrap_or_default();
        Ok(Arc::new(FakeProcessHandle::new(plan)))
    }
}

struct FakeProcessHandle {
    plan: Mutex<FakePlan>,
    exit_listener: Mutex<Option<ExitListener>>,
    _message_listener: Mutex<Option<MessageListener>>,
}

impl FakeProcessHandle {
    fn new(plan: FakePlan) -> Self {
        Self {
            plan: Mutex::new(plan),
            exit_listener: Mutex::new(None),
            _message_listener: Mutex::new(None),
        }
    }

    fn notify_exit(&self, code: i32) {
        if let Some(listener) = self.exit_listener.lock().expect("listener").clone() {
            listener(ProcessExitInfo { code: Some(code) });
        }
    }
}

impl PluginProcessHandle for FakeProcessHandle {
    fn process_id(&self) -> Option<u32> {
        Some(42)
    }

    fn wait_for_ready(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").ready.clone() {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "ready timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn activate(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").activate.clone() {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "activate timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn request(
        &self,
        method: &str,
        payload: Value,
        _timeout: Duration,
    ) -> Result<WireMessage, PluginRuntimeError> {
        let requests_seen = self.plan.lock().expect("plan").requests_seen.clone();
        requests_seen
            .lock()
            .expect("requests seen")
            .push((method.to_string(), payload));
        let outcome = self
            .plan
            .lock()
            .expect("plan")
            .requests
            .get(method)
            .cloned()
            .unwrap_or(FakeRequestOutcome::Error("UNHANDLED", "no handler"));
        match outcome {
            FakeRequestOutcome::Success(payload) => Ok(WireMessage {
                message_type: WireMessageType::Response,
                id: "response".to_string(),
                method: method.to_string(),
                payload: Some(payload),
                error: None,
            }),
            FakeRequestOutcome::Error(code, message) => Ok(WireMessage {
                message_type: WireMessageType::Response,
                id: "response".to_string(),
                method: method.to_string(),
                payload: None,
                error: Some(WireError {
                    code: code.to_string(),
                    message: message.to_string(),
                }),
            }),
            FakeRequestOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "call timeout".to_string(),
            }),
            FakeRequestOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn send_event(&self, method: &str, payload: Value) -> Result<(), PluginRuntimeError> {
        let sent_events = self.plan.lock().expect("plan").sent_events.clone();
        sent_events
            .lock()
            .expect("sent events")
            .push((method.to_string(), payload));
        Ok(())
    }

    fn heartbeat(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        let outcome = {
            let mut plan = self.plan.lock().expect("plan");
            if plan.heartbeats.is_empty() {
                FakeOutcome::Ok
            } else {
                plan.heartbeats.remove(0)
            }
        };
        match outcome {
            FakeOutcome::Ok => Ok(()),
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: PLUGIN_HEARTBEAT_TIMEOUT_CODE.to_string(),
                message: "heartbeat timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn deactivate(&self, _timeout: Duration) -> Result<(), PluginRuntimeError> {
        match self.plan.lock().expect("plan").deactivate.clone() {
            FakeOutcome::Ok => {
                self.notify_exit(0);
                Ok(())
            }
            FakeOutcome::Timeout => Err(PluginRuntimeError {
                code: "TIMEOUT".to_string(),
                message: "deactivate timeout".to_string(),
            }),
            FakeOutcome::Error(message) => Err(PluginRuntimeError {
                code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                message: message.to_string(),
            }),
            FakeOutcome::Crash(code) => {
                self.notify_exit(code);
                Err(PluginRuntimeError {
                    code: PLUGIN_RUNTIME_ERROR_CODE.to_string(),
                    message: "process crashed".to_string(),
                })
            }
        }
    }

    fn kill(&self) -> Result<(), PluginRuntimeError> {
        let killed = self.plan.lock().expect("plan").killed.clone();
        killed.store(true, Ordering::Relaxed);
        self.notify_exit(-1);
        Ok(())
    }

    fn set_exit_listener(&self, listener: Arc<dyn Fn(ProcessExitInfo) + Send + Sync>) {
        *self.exit_listener.lock().expect("listener") = Some(listener);
    }

    fn set_message_listener(
        &self,
        listener: Arc<dyn Fn(WireMessage) -> Option<WireMessage> + Send + Sync>,
    ) {
        *self._message_listener.lock().expect("message listener") = Some(listener);
    }

    fn stderr_snapshot(&self) -> Option<String> {
        None
    }
}
