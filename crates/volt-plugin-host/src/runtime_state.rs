use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, BufReader};

use serde_json::Value;

use crate::config::{DelegatedGrant, HostIpcSettings, PluginConfig};
use crate::ipc::{IpcMessage, MessageType, read_message, write_message};

struct RuntimeState {
    plugin_id: String,
    manifest: Value,
    delegated_grants: Vec<DelegatedGrant>,
    max_deferred_messages: usize,
    transport: Transport,
}

enum Transport {
    StdIo {
        reader: BufReader<std::io::Stdin>,
        writer: std::io::Stdout,
        deferred: VecDeque<IpcMessage>,
        next_id: u64,
    },
    #[cfg(test)]
    Mock {
        inbound: VecDeque<IpcMessage>,
        outbound: Vec<IpcMessage>,
        deferred: VecDeque<IpcMessage>,
        next_id: u64,
    },
}

thread_local! {
    static STATE: RefCell<Option<RuntimeState>> = const { RefCell::new(None) };
}

pub fn configure_stdio(config: &PluginConfig) {
    let host_ipc_settings = config.host_ipc_settings.clone().unwrap_or_default();
    STATE.with(|state| {
        *state.borrow_mut() = Some(RuntimeState {
            plugin_id: config.plugin_id.clone(),
            manifest: config.manifest.clone(),
            delegated_grants: config.delegated_grants.clone(),
            max_deferred_messages: max_deferred_messages(&host_ipc_settings),
            transport: Transport::StdIo {
                reader: BufReader::new(std::io::stdin()),
                writer: std::io::stdout(),
                deferred: VecDeque::new(),
                next_id: 1,
            },
        });
    });
}

pub fn manifest() -> Result<Value, String> {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|runtime| runtime.manifest.clone())
            .ok_or_else(|| "plugin runtime is not configured".to_string())
    })
}

pub fn plugin_id() -> Result<String, String> {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|runtime| runtime.plugin_id.clone())
            .ok_or_else(|| "plugin runtime is not configured".to_string())
    })
}

pub fn delegated_grants() -> Result<Vec<DelegatedGrant>, String> {
    STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .map(|runtime| runtime.delegated_grants.clone())
            .ok_or_else(|| "plugin runtime is not configured".to_string())
    })
}

pub fn next_message() -> io::Result<Option<IpcMessage>> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let runtime = state
            .as_mut()
            .ok_or_else(|| io::Error::other("plugin runtime is not configured"))?;
        runtime.transport.next_message()
    })
}

pub fn send_signal(id: impl Into<String>, method: impl Into<String>) -> Result<(), String> {
    send_message(IpcMessage::signal(id, method))
}

pub fn send_event(method: impl Into<String>, payload: Value) -> Result<(), String> {
    send_message(IpcMessage {
        msg_type: MessageType::Event,
        id: "event".to_string(),
        method: method.into(),
        payload: Some(payload),
        error: None,
    })
}

pub fn send_response(
    id: impl Into<String>,
    method: impl Into<String>,
    payload: Option<Value>,
) -> Result<(), String> {
    send_message(IpcMessage::response(id, method, payload))
}

pub fn send_error(
    id: impl Into<String>,
    method: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Result<(), String> {
    send_message(IpcMessage::error_response(id, method, code, message))
}

pub fn send_request(method: impl Into<String>, payload: Value) -> Result<Value, String> {
    let method = method.into();
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let runtime = state
            .as_mut()
            .ok_or_else(|| "plugin runtime is not configured".to_string())?;
        let request_id = runtime.transport.next_request_id();
        runtime.transport.write(&IpcMessage {
            msg_type: MessageType::Request,
            id: request_id.clone(),
            method,
            payload: Some(payload),
            error: None,
        })?;

        loop {
            let Some(message) = runtime.transport.read_backend_only()? else {
                return Err("host closed the plugin transport".to_string());
            };

            if message.msg_type == MessageType::Response && message.id == request_id {
                if let Some(error) = message.error {
                    return Err(format!("{}: {}", error.code, error.message));
                }
                return Ok(message.payload.unwrap_or(Value::Null));
            }

            if message.msg_type == MessageType::Signal && message.method == "heartbeat" {
                runtime
                    .transport
                    .write(&IpcMessage::signal(message.id, "heartbeat-ack"))?;
                continue;
            }

            runtime
                .transport
                .defer(message, runtime.max_deferred_messages)?;
        }
    })
}

fn send_message(message: IpcMessage) -> Result<(), String> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let runtime = state
            .as_mut()
            .ok_or_else(|| "plugin runtime is not configured".to_string())?;
        runtime.transport.write(&message)
    })
}

impl Transport {
    fn next_request_id(&mut self) -> String {
        match self {
            Self::StdIo { next_id, .. } => {
                let id = format!("plugin-request-{next_id}");
                *next_id = next_id.saturating_add(1);
                id
            }
            #[cfg(test)]
            Self::Mock { next_id, .. } => {
                let id = format!("plugin-request-{next_id}");
                *next_id = next_id.saturating_add(1);
                id
            }
        }
    }

    fn defer(&mut self, message: IpcMessage, max_deferred_messages: usize) -> Result<(), String> {
        match self {
            Self::StdIo { deferred, .. } => {
                if deferred.len() >= max_deferred_messages {
                    return Err(format!(
                        "host deferred message queue exceeded {} messages while waiting for a synchronous response",
                        max_deferred_messages
                    ));
                }
                deferred.push_back(message);
                Ok(())
            }
            #[cfg(test)]
            Self::Mock { deferred, .. } => {
                if deferred.len() >= max_deferred_messages {
                    return Err(format!(
                        "host deferred message queue exceeded {} messages while waiting for a synchronous response",
                        max_deferred_messages
                    ));
                }
                deferred.push_back(message);
                Ok(())
            }
        }
    }

    fn next_message(&mut self) -> io::Result<Option<IpcMessage>> {
        match self {
            Self::StdIo {
                reader, deferred, ..
            } => {
                if let Some(message) = deferred.pop_front() {
                    return Ok(Some(message));
                }
                read_message(reader)
            }
            #[cfg(test)]
            Self::Mock {
                inbound, deferred, ..
            } => {
                if let Some(message) = deferred.pop_front() {
                    return Ok(Some(message));
                }
                Ok(inbound.pop_front())
            }
        }
    }

    fn read_backend_only(&mut self) -> Result<Option<IpcMessage>, String> {
        match self {
            Self::StdIo { reader, .. } => read_message(reader).map_err(|error| error.to_string()),
            #[cfg(test)]
            Self::Mock { inbound, .. } => Ok(inbound.pop_front()),
        }
    }

    fn write(&mut self, message: &IpcMessage) -> Result<(), String> {
        match self {
            Self::StdIo { writer, .. } => {
                write_message(writer, message).map_err(|error| error.to_string())
            }
            #[cfg(test)]
            Self::Mock { outbound, .. } => {
                outbound.push(message.clone());
                Ok(())
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn configure_mock(config: &PluginConfig, inbound: Vec<IpcMessage>) {
    let host_ipc_settings = config.host_ipc_settings.clone().unwrap_or_default();
    STATE.with(|state| {
        *state.borrow_mut() = Some(RuntimeState {
            plugin_id: config.plugin_id.clone(),
            manifest: config.manifest.clone(),
            delegated_grants: config.delegated_grants.clone(),
            max_deferred_messages: max_deferred_messages(&host_ipc_settings),
            transport: Transport::Mock {
                inbound: inbound.into(),
                outbound: Vec::new(),
                deferred: VecDeque::new(),
                next_id: 1,
            },
        });
    });
}

#[cfg(test)]
pub(crate) fn take_outbound() -> Vec<IpcMessage> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let runtime = state.as_mut().expect("runtime configured");
        match &mut runtime.transport {
            Transport::Mock { outbound, .. } => std::mem::take(outbound),
            Transport::StdIo { .. } => panic!("mock transport expected"),
        }
    })
}

fn max_deferred_messages(settings: &HostIpcSettings) -> usize {
    settings.max_queue_depth.max(1) as usize
}

#[cfg(test)]
mod tests;
