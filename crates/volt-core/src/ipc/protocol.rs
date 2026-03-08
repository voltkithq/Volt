use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const IPC_HANDLER_NOT_FOUND_CODE: &str = "IPC_HANDLER_NOT_FOUND";
pub const IPC_HANDLER_ERROR_CODE: &str = "IPC_HANDLER_ERROR";
pub const IPC_HANDLER_TIMEOUT_CODE: &str = "IPC_HANDLER_TIMEOUT";

/// An incoming IPC message from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    /// Unique request ID for matching responses.
    pub id: String,
    /// Handler method name.
    pub method: String,
    /// Arguments payload.
    #[serde(default)]
    pub args: serde_json::Value,
}

/// A response to an IPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcResponse {
    /// Matching request ID.
    pub id: String,
    /// Result payload (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Stable error code (present on structured failures).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Optional structured details for the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<serde_json::Value>,
}

impl IpcResponse {
    /// Create a success response.
    pub fn success(id: String, result: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
            error_code: None,
            error_details: None,
        }
    }

    /// Create an error response.
    pub fn error(id: String, message: String) -> Self {
        Self::error_with_code(id, message, IPC_HANDLER_ERROR_CODE.to_string())
    }

    /// Create an error response with a stable error code.
    pub fn error_with_code(id: String, message: String, code: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(message),
            error_code: Some(code),
            error_details: None,
        }
    }

    /// Create an error response with code and structured details.
    pub fn error_with_details(
        id: String,
        message: String,
        code: String,
        details: serde_json::Value,
    ) -> Self {
        Self {
            id,
            result: None,
            error: Some(message),
            error_code: Some(code),
            error_details: Some(details),
        }
    }
}

/// Type alias for IPC handler functions.
/// Handler receives JSON args and returns JSON result or error string.
pub type HandlerFn =
    Arc<dyn Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;
