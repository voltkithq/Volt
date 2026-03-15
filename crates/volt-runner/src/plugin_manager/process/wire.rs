use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WireMessageType {
    Request,
    Response,
    Event,
    Signal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WireError {
    pub(crate) code: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct WireMessage {
    #[serde(rename = "type")]
    pub(crate) message_type: WireMessageType,
    pub(crate) id: String,
    pub(crate) method: String,
    pub(crate) payload: Option<Value>,
    pub(crate) error: Option<WireError>,
}

impl WireMessage {
    pub(crate) fn request(id: String, method: impl Into<String>, payload: Value) -> Self {
        Self {
            message_type: WireMessageType::Request,
            id,
            method: method.into(),
            payload: Some(payload),
            error: None,
        }
    }

    pub(crate) fn signal(id: String, method: impl Into<String>, payload: Option<Value>) -> Self {
        Self {
            message_type: WireMessageType::Signal,
            id,
            method: method.into(),
            payload,
            error: None,
        }
    }
}
