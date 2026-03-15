use std::collections::BTreeMap;
use std::time::Duration;

use boa_engine::{Context, JsValue};
use reqwest::Method;
use serde_json::Value;

use crate::modules::value_to_json;

use super::allow_private_networks_for_tests;
use super::constants::{
    HTTP_DEFAULT_TIMEOUT, HTTP_MAX_HEADER_COUNT, HTTP_MAX_HEADER_NAME_BYTES,
    HTTP_MAX_HEADER_VALUE_BYTES, HTTP_MAX_REQUEST_BODY_BYTES, HTTP_MAX_TIMEOUT_MS,
};
use super::ssrf::normalize_request_url;

#[derive(Debug, Clone)]
pub(crate) struct HttpRequest {
    pub(crate) url: String,
    pub(crate) method: Method,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Option<String>,
    pub(crate) timeout: Duration,
}

pub(super) fn parse_fetch_request(
    input: JsValue,
    options: Option<JsValue>,
    context: &mut Context,
) -> Result<HttpRequest, String> {
    let input = value_to_json(input, context)?;
    let options = options
        .map(|value| value_to_json(value, context))
        .transpose()?;

    parse_fetch_request_json(input, options, allow_private_networks_for_tests())
}

pub(crate) fn parse_fetch_request_json(
    input: Value,
    options: Option<Value>,
    allow_private_networks: bool,
) -> Result<HttpRequest, String> {
    match input {
        Value::String(url) => parse_legacy_fetch_request(url, options.as_ref(), allow_private_networks),
        Value::Object(object) => {
            parse_request_object_fetch_request(object, options.as_ref(), allow_private_networks)
        }
        _ => Err(
            "http.fetch expects either (url, options?) or a request object with a string 'url' field"
                .to_string(),
        ),
    }
}

fn parse_legacy_fetch_request(
    url: String,
    options: Option<&Value>,
    allow_private_networks: bool,
) -> Result<HttpRequest, String> {
    Ok(HttpRequest {
        url: normalize_request_url(&url, allow_private_networks)?,
        method: parse_method(options)?,
        headers: parse_headers(options)?,
        body: parse_body(options)?,
        timeout: parse_timeout(options)?,
    })
}

fn parse_request_object_fetch_request(
    object: serde_json::Map<String, Value>,
    options: Option<&Value>,
    allow_private_networks: bool,
) -> Result<HttpRequest, String> {
    if let Some(options) = options.filter(|value| !value.is_null()) {
        return Err(format!(
            "http.fetch request objects do not accept a second argument (received {options})"
        ));
    }

    let request = Value::Object(object);
    let url = request
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "http.fetch request.url must be a string".to_string())?;

    Ok(HttpRequest {
        url: normalize_request_url(url, allow_private_networks)?,
        method: parse_method(Some(&request))?,
        headers: parse_headers(Some(&request))?,
        body: parse_body(Some(&request))?,
        timeout: parse_timeout(Some(&request))?,
    })
}

fn parse_method(options: Option<&Value>) -> Result<Method, String> {
    let method = options
        .and_then(|value| value.get("method"))
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .trim()
        .to_uppercase();

    Method::from_bytes(method.as_bytes()).map_err(|err| format!("invalid HTTP method: {err}"))
}

fn parse_timeout(options: Option<&Value>) -> Result<Duration, String> {
    let Some(timeout_ms) = options.and_then(|value| value.get("timeoutMs")) else {
        return Ok(HTTP_DEFAULT_TIMEOUT);
    };

    let Some(timeout_ms) = timeout_ms.as_u64() else {
        return Err("http.fetch timeoutMs must be a positive integer".to_string());
    };
    if timeout_ms == 0 {
        return Err("http.fetch timeoutMs must be greater than zero".to_string());
    }
    if timeout_ms > HTTP_MAX_TIMEOUT_MS {
        return Err(format!(
            "http.fetch timeoutMs must be <= {HTTP_MAX_TIMEOUT_MS}"
        ));
    }

    Ok(Duration::from_millis(timeout_ms))
}

pub(crate) fn parse_headers(options: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
    let mut headers = BTreeMap::new();
    let Some(header_map) = options
        .and_then(|value| value.get("headers"))
        .and_then(Value::as_object)
    else {
        return Ok(headers);
    };
    if header_map.len() > HTTP_MAX_HEADER_COUNT {
        return Err(format!(
            "http.fetch headers exceed limit ({})",
            HTTP_MAX_HEADER_COUNT
        ));
    }

    for (name, value) in header_map {
        let value = value
            .as_str()
            .ok_or_else(|| format!("header '{name}' must be a string"))?;
        validate_header(name, value)?;
        headers.insert(name.clone(), value.to_string());
    }

    Ok(headers)
}

pub(crate) fn parse_body(options: Option<&Value>) -> Result<Option<String>, String> {
    let Some(body_value) = options.and_then(|value| value.get("body")) else {
        return Ok(None);
    };

    if body_value.is_null() {
        return Ok(None);
    }
    if let Some(as_string) = body_value.as_str() {
        validate_request_body_size(as_string.len())?;
        return Ok(Some(as_string.to_string()));
    }

    let serialized = serde_json::to_string(body_value)
        .map_err(|err| format!("failed to serialize request body: {err}"))?;
    validate_request_body_size(serialized.len())?;
    Ok(Some(serialized))
}

fn validate_request_body_size(size: usize) -> Result<(), String> {
    if size > HTTP_MAX_REQUEST_BODY_BYTES {
        return Err(format!(
            "http.fetch request body exceeds {} bytes",
            HTTP_MAX_REQUEST_BODY_BYTES
        ));
    }
    Ok(())
}

fn validate_header(name: &str, value: &str) -> Result<(), String> {
    if name.len() > HTTP_MAX_HEADER_NAME_BYTES {
        return Err(format!(
            "header '{name}' exceeds {} bytes",
            HTTP_MAX_HEADER_NAME_BYTES
        ));
    }
    if value.len() > HTTP_MAX_HEADER_VALUE_BYTES {
        return Err(format!(
            "header '{name}' value exceeds {} bytes",
            HTTP_MAX_HEADER_VALUE_BYTES
        ));
    }

    reqwest::header::HeaderName::from_bytes(name.as_bytes())
        .map_err(|error| format!("invalid header name '{name}': {error}"))?;
    reqwest::header::HeaderValue::from_str(value)
        .map_err(|error| format!("invalid header value for '{name}': {error}"))?;
    Ok(())
}
