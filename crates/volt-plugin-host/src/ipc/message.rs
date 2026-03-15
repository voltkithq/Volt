use serde::{Deserialize, Serialize};

/// Message types in the plugin-host IPC protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Request,
    Response,
    Event,
    Signal,
}

/// Error payload in an IPC message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

/// A single IPC message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub id: String,
    pub method: String,
    pub payload: Option<serde_json::Value>,
    pub error: Option<IpcError>,
}

impl IpcMessage {
    pub fn signal(id: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Signal,
            id: id.into(),
            method: method.into(),
            payload: None,
            error: None,
        }
    }

    pub fn response(
        id: impl Into<String>,
        method: impl Into<String>,
        payload: Option<serde_json::Value>,
    ) -> Self {
        Self {
            msg_type: MessageType::Response,
            id: id.into(),
            method: method.into(),
            payload,
            error: None,
        }
    }

    pub fn error_response(
        id: impl Into<String>,
        method: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            msg_type: MessageType::Response,
            id: id.into(),
            method: method.into(),
            payload: None,
            error: Some(IpcError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}
