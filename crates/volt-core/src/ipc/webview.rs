use serde_json::Value as JsonValue;

use super::{IPC_MAX_REQUEST_BYTES, IpcError, IpcResponse};

/// Generate the JavaScript IPC initialization script injected into the WebView.
/// This creates the `window.__volt__` API and the pending request tracking.
pub fn ipc_init_script() -> String {
    r#"
(function() {
    'use strict';

    // Pending request map: id -> { resolve, reject }
    const pending = new Map();
    const MAX_PENDING_REQUESTS = 128;
    const MAX_PAYLOAD_BYTES = 262144;
    const encodeUtf8 = typeof TextEncoder !== 'undefined'
        ? function(text) { return new TextEncoder().encode(text).length; }
        : function(text) { return text.length; };

    // Event listener map: event -> Set<callback>
    const listeners = new Map();

    // IPC API exposed to the renderer
    window.__volt__ = Object.freeze({
        invoke: function(method, args) {
            return new Promise(function(resolve, reject) {
                if (pending.size >= MAX_PENDING_REQUESTS) {
                    var inflightError = new Error('IPC in-flight limit reached: ' + MAX_PENDING_REQUESTS);
                    inflightError.code = 'IPC_IN_FLIGHT_LIMIT';
                    inflightError.details = { maxInFlight: MAX_PENDING_REQUESTS };
                    reject(inflightError);
                    return;
                }
                const id = crypto.randomUUID();
                const payload = JSON.stringify({
                    id: id,
                    method: method,
                    args: args !== undefined ? args : null
                });
                const payloadBytes = encodeUtf8(payload);
                if (payloadBytes > MAX_PAYLOAD_BYTES) {
                    var payloadError = new Error('IPC payload too large (' + payloadBytes + ' bytes > ' + MAX_PAYLOAD_BYTES + ' bytes)');
                    payloadError.code = 'IPC_PAYLOAD_TOO_LARGE';
                    payloadError.details = { payloadBytes: payloadBytes, maxPayloadBytes: MAX_PAYLOAD_BYTES };
                    reject(payloadError);
                    return;
                }

                pending.set(id, { resolve: resolve, reject: reject });
                window.ipc.postMessage(payload);
                // Frontend timeout for IPC responses. This is a safety net for
                // lost responses — dialogs bypass the Boa runtime but the
                // frontend can't distinguish dialog calls, so this must
                // accommodate user interaction time (e.g. file picker).
                setTimeout(function() {
                    if (pending.has(id)) {
                        pending.delete(id);
                        reject(new Error('IPC request timed out: ' + method));
                    }
                }, 120000);
            });
        },
        on: function(event, callback) {
            if (!listeners.has(event)) {
                listeners.set(event, new Set());
            }
            listeners.get(event).add(callback);
        },
        off: function(event, callback) {
            var set = listeners.get(event);
            if (set) {
                set.delete(callback);
            }
        }
    });

    // Response handler called from Rust via evaluate_script.
    // Defined as non-writable to prevent interception by injected scripts.
    Object.defineProperty(window, '__volt_response__', { value: function(responseJson) {
        try {
            var response = JSON.parse(responseJson);
            var p = pending.get(response.id);
            if (p) {
                pending.delete(response.id);
                if (response.error) {
                    var error = new Error(response.error);
                    if (response.errorCode) {
                        error.code = response.errorCode;
                    }
                    if (response.errorDetails !== undefined) {
                        error.details = response.errorDetails;
                    }
                    p.reject(error);
                } else {
                    p.resolve(response.result);
                }
            }
        } catch (e) {
            console.error('[volt] Failed to parse IPC response:', e);
        }
    }, writable: false, configurable: false });

    // Event handler called from Rust via evaluate_script.
    // Defined as non-writable to prevent interception by injected scripts.
    Object.defineProperty(window, '__volt_event__', { value: function(event, data) {
        var set = listeners.get(event);
        if (set) {
            set.forEach(function(cb) {
                try {
                    cb(data);
                } catch (e) {
                    console.error('[volt] Event handler error:', e);
                }
            });
        }
    }, writable: false, configurable: false });

    // Forward CSP violations to native logging so they are visible without DevTools.
    document.addEventListener('securitypolicyviolation', function(e) {
        var msg = '[volt:csp] Blocked ' + e.blockedURI +
            ' — violates "' + e.violatedDirective + '" directive.';
        console.error(msg);
        try {
            window.ipc.postMessage(JSON.stringify({
                id: '__csp_violation__',
                method: '__volt_internal:csp-violation',
                args: {
                    blockedURI: e.blockedURI,
                    violatedDirective: e.violatedDirective,
                    effectiveDirective: e.effectiveDirective,
                    originalPolicy: e.originalPolicy
                }
            }));
        } catch(_) {}
    });
})();
"#
    .to_string()
}

/// Generate the JavaScript code to deliver an IPC response to the frontend.
/// Properly escapes the JSON string to prevent injection.
pub fn response_script(response_json: &str) -> String {
    // Escape the JSON for embedding in a JavaScript single-quoted string literal.
    // We use JSON.parse in the script to avoid any injection via the payload.
    let escaped = escape_for_single_quoted_js(response_json);
    format!("window.__volt_response__('{escaped}')")
}

/// Generate an IPC error response script for an oversized payload that was
/// rejected before it entered the native dispatch queue.
pub fn payload_too_large_response_script(raw: &str) -> String {
    let payload_bytes = raw.len();
    let response = IpcResponse::error_with_details(
        extract_request_id(raw),
        format!(
            "IPC payload too large ({} bytes > {} bytes)",
            payload_bytes, IPC_MAX_REQUEST_BYTES
        ),
        "IPC_PAYLOAD_TOO_LARGE".to_string(),
        serde_json::json!({
            "payloadBytes": payload_bytes,
            "maxPayloadBytes": IPC_MAX_REQUEST_BYTES
        }),
    );
    let response_json = serde_json::to_string(&response)
        .expect("serializing structured IPC payload-too-large response should not fail");
    response_script(&response_json)
}

/// Generate the JavaScript code to emit an event to the frontend.
pub fn event_script(event_name: &str, data: &serde_json::Value) -> Result<String, IpcError> {
    let data_json = serde_json::to_string(data)
        .map_err(|e| IpcError::HandlerError(format!("Failed to serialize event data: {e}")))?;
    let escaped_data = escape_for_single_quoted_js(&data_json);
    let escaped_name = escape_for_single_quoted_js(event_name);
    Ok(format!(
        "window.__volt_event__('{escaped_name}', JSON.parse('{escaped_data}'))"
    ))
}

fn escape_for_single_quoted_js(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\'' => escaped.push_str("\\'"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\0' => escaped.push_str("\\0"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn extract_request_id(raw: &str) -> String {
    serde_json::from_str::<JsonValue>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("id")
                .and_then(JsonValue::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}
