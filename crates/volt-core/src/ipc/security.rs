use thiserror::Error;

/// Errors that can occur during IPC operations.
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("handler not found: {0}")]
    HandlerNotFound(String),

    #[error("invalid JSON message: {0}")]
    InvalidMessage(String),

    #[error("prototype pollution attempt detected")]
    PrototypePollution,

    #[error("rate limit exceeded: {0} requests/second")]
    RateLimitExceeded(u32),

    #[error("handler execution failed: {0}")]
    HandlerError(String),

    #[error("security error: {0}")]
    Security(String),

    #[error("payload too large: {size} bytes (max {max})")]
    PayloadTooLarge { size: usize, max: usize },
}

pub const IPC_MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_PROTOTYPE_CHECK_DEPTH: usize = 64;

/// Parse raw JSON and reject payloads containing prototype-pollution keys.
///
/// This inspects object keys only; string values containing words like
/// "constructor" are valid and must not be rejected.
pub(super) fn check_prototype_pollution(raw: &str) -> Result<serde_json::Value, IpcError> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| IpcError::InvalidMessage(e.to_string()))?;
    check_value_prototype_pollution(&value, 0)?;
    Ok(value)
}

/// Recursively check a serde_json::Value for prototype pollution keys.
fn check_value_prototype_pollution(
    value: &serde_json::Value,
    depth: usize,
) -> Result<(), IpcError> {
    if depth > MAX_PROTOTYPE_CHECK_DEPTH {
        return Err(IpcError::Security(format!(
            "payload nesting exceeds max depth ({MAX_PROTOTYPE_CHECK_DEPTH})"
        )));
    }

    match value {
        serde_json::Value::Object(map) => {
            for key in map.keys() {
                if key == "__proto__" || key == "constructor" || key == "prototype" {
                    return Err(IpcError::PrototypePollution);
                }
            }
            for v in map.values() {
                check_value_prototype_pollution(v, depth + 1)?;
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                check_value_prototype_pollution(v, depth + 1)?;
            }
        }
        _ => {}
    }

    Ok(())
}
