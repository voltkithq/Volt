mod protocol;
mod rate_limit;
mod security;
mod webview;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use self::security::check_prototype_pollution;

pub use self::protocol::{
    HandlerFn, IPC_HANDLER_ERROR_CODE, IPC_HANDLER_NOT_FOUND_CODE, IPC_HANDLER_TIMEOUT_CODE,
    IpcRequest, IpcResponse,
};
pub use self::rate_limit::RateLimiter;
pub use self::security::{IPC_MAX_REQUEST_BYTES, IpcError};
pub use self::webview::{
    IPC_MAX_RESPONSE_BYTES, event_script, ipc_init_script, payload_too_large_response_script,
    response_script,
};

/// Registry of IPC handlers mapped by method name.
pub struct IpcRegistry {
    handlers: Arc<Mutex<HashMap<String, HandlerFn>>>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl IpcRegistry {
    /// Create a new IPC registry with the default rate limit (1000 req/s).
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(1000, Duration::from_secs(1)))),
        }
    }

    /// Register a handler for a method name.
    pub fn register<F>(&self, method: &str, handler: F) -> Result<(), IpcError>
    where
        F: Fn(serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync + 'static,
    {
        let mut handlers = self
            .handlers
            .lock()
            .map_err(|e| IpcError::HandlerError(format!("Lock poisoned: {e}")))?;
        handlers.insert(method.to_string(), Arc::new(handler));
        Ok(())
    }

    /// Remove a registered handler.
    pub fn remove_handler(&self, method: &str) -> Result<(), IpcError> {
        let mut handlers = self
            .handlers
            .lock()
            .map_err(|e| IpcError::HandlerError(format!("Lock poisoned: {e}")))?;
        handlers.remove(method);
        Ok(())
    }

    /// Remove all registered handlers.
    pub fn clear_handlers(&self) -> Result<(), IpcError> {
        let mut handlers = self
            .handlers
            .lock()
            .map_err(|e| IpcError::HandlerError(format!("Lock poisoned: {e}")))?;
        handlers.clear();
        Ok(())
    }

    /// Handle an incoming raw IPC message string.
    /// Returns the JSON response string to send back to the frontend.
    pub fn handle_message(&self, raw: &str) -> Result<String, IpcError> {
        if raw.len() > IPC_MAX_REQUEST_BYTES {
            return Err(IpcError::PayloadTooLarge {
                size: raw.len(),
                max: IPC_MAX_REQUEST_BYTES,
            });
        }

        {
            let mut limiter = self
                .rate_limiter
                .lock()
                .map_err(|e| IpcError::HandlerError(format!("Lock poisoned: {e}")))?;
            limiter.check()?;
        }

        let raw_value = check_prototype_pollution(raw)?;
        let request: IpcRequest = serde_json::from_value(raw_value)
            .map_err(|e| IpcError::InvalidMessage(e.to_string()))?;

        let handler = {
            let handlers = self
                .handlers
                .lock()
                .map_err(|e| IpcError::HandlerError(format!("Lock poisoned: {e}")))?;
            handlers.get(&request.method).cloned()
        };

        let response = match handler {
            Some(handler) => match handler(request.args) {
                Ok(result) => IpcResponse::success(request.id, result),
                Err(err) => IpcResponse::error_with_code(
                    request.id,
                    err,
                    IPC_HANDLER_ERROR_CODE.to_string(),
                ),
            },
            None => IpcResponse::error_with_code(
                request.id,
                format!("Handler not found: {}", request.method),
                IPC_HANDLER_NOT_FOUND_CODE.to_string(),
            ),
        };

        serde_json::to_string(&response)
            .map_err(|e| IpcError::HandlerError(format!("Failed to serialize response: {e}")))
    }
}

impl Default for IpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
