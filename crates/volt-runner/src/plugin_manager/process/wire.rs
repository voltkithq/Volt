use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(in crate::plugin_manager) enum WireMessageType {
    Request,
    Response,
    Event,
    Signal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(in crate::plugin_manager) struct WireError {
    pub(in crate::plugin_manager) code: String,
    pub(in crate::plugin_manager) message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(in crate::plugin_manager) struct WireMessage {
    #[serde(rename = "type")]
    pub(in crate::plugin_manager) message_type: WireMessageType,
    pub(in crate::plugin_manager) id: String,
    pub(in crate::plugin_manager) method: String,
    pub(in crate::plugin_manager) payload: Option<Value>,
    pub(in crate::plugin_manager) error: Option<WireError>,
}

impl WireMessage {
    pub(in crate::plugin_manager) fn request(
        id: String,
        method: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            message_type: WireMessageType::Request,
            id,
            method: method.into(),
            payload: Some(payload),
            error: None,
        }
    }

    pub(in crate::plugin_manager) fn signal(
        id: String,
        method: impl Into<String>,
        payload: Option<Value>,
    ) -> Self {
        Self {
            message_type: WireMessageType::Signal,
            id,
            method: method.into(),
            payload,
            error: None,
        }
    }
}
