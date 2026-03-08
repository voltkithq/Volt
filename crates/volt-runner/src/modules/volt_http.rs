mod ssrf;

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use boa_engine::native_function::NativeFunction;
use boa_engine::object::{JsObject, ObjectInitializer};
use boa_engine::property::Attribute;
use boa_engine::{Context, IntoJsFunctionCopied, JsError, JsResult, JsValue, Module, js_string};
use reqwest::Method;
use reqwest::blocking::ClientBuilder;
use reqwest::redirect::Policy;
use serde_json::Value;
use volt_core::permissions::Permission;

use super::{
    format_js_error, json_to_js_value, native_function_module, promise_from_result,
    require_permission, resolve_promise, value_to_json,
};

type ResponseHeaders = BTreeMap<String, Vec<String>>;
type FetchResult = (i32, ResponseHeaders, String);

const HTTP_DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_MAX_TIMEOUT_MS: u64 = 120_000;
const HTTP_MAX_RESPONSE_BODY_BYTES: usize = 2 * 1024 * 1024;
const HTTP_MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;
const HTTP_MAX_CONCURRENT_REQUESTS: usize = 32;
const HTTP_MAX_HEADER_COUNT: usize = 64;
const HTTP_MAX_HEADER_NAME_BYTES: usize = 128;
const HTTP_MAX_HEADER_VALUE_BYTES: usize = 8 * 1024;
const HTTP_MAX_REDIRECTS: usize = 10;
const RESPONSE_BODY_PROPERTY: &str = "__voltBody";

static HTTP_REQUESTS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
struct HttpRequest {
    url: String,
    method: Method,
    headers: BTreeMap<String, String>,
    body: Option<String>,
    timeout: Duration,
}

struct InFlightRequestGuard;

impl InFlightRequestGuard {
    fn try_acquire() -> Result<Self, String> {
        if try_acquire_inflight_slot(&HTTP_REQUESTS_IN_FLIGHT, HTTP_MAX_CONCURRENT_REQUESTS) {
            Ok(Self)
        } else {
            Err(format!(
                "too many concurrent HTTP requests (limit: {HTTP_MAX_CONCURRENT_REQUESTS})"
            ))
        }
    }
}

impl Drop for InFlightRequestGuard {
    fn drop(&mut self) {
        HTTP_REQUESTS_IN_FLIGHT.fetch_sub(1, Ordering::AcqRel);
    }
}

fn try_acquire_inflight_slot(counter: &AtomicUsize, max: usize) -> bool {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            if current >= max {
                None
            } else {
                Some(current + 1)
            }
        })
        .is_ok()
}

fn fetch(input: JsValue, options: Option<JsValue>, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Http).map_err(format_js_error)?;
        let request = parse_fetch_request(input, options, context)?;
        fetch_impl(request, context)
    })();

    promise_from_result(context, result).into()
}

fn fetch_impl(request: HttpRequest, context: &mut Context) -> Result<JsObject, String> {
    let (status, response_headers, response_body) = perform_http_request(request)?;

    build_response_object(context, status, response_headers, response_body)
}

fn perform_http_request(request: HttpRequest) -> Result<FetchResult, String> {
    let _request_guard = InFlightRequestGuard::try_acquire()?;
    let allow_private = allow_private_networks_for_tests();

    let worker = std::thread::spawn(move || execute_with_dns_pinning(request, allow_private));

    match worker.join() {
        Ok(result) => result,
        Err(_) => Err("HTTP request worker thread panicked".to_string()),
    }
}

/// Execute an HTTP request with DNS pinning to prevent DNS rebinding TOCTOU.
///
/// Instead of letting reqwest resolve DNS independently (which could yield a
/// different IP than the one we validated), we resolve DNS ourselves, validate
/// all returned addresses, then pin the first valid address to the client via
/// `ClientBuilder::resolve()`. Redirects are followed manually so each hop
/// gets the same resolve-validate-pin treatment.
fn execute_with_dns_pinning(
    request: HttpRequest,
    allow_private: bool,
) -> Result<FetchResult, String> {
    let mut current_url: reqwest::Url =
        reqwest::Url::parse(&request.url).map_err(|e| format!("invalid request URL: {e}"))?;
    let mut remaining_redirects = HTTP_MAX_REDIRECTS;

    loop {
        let host = current_url
            .host_str()
            .ok_or_else(|| "request URL must include a host".to_string())?
            .to_string();
        let port = current_url
            .port_or_known_default()
            .ok_or_else(|| "request URL must include a valid port".to_string())?;

        let resolved_addr = resolve_and_validate_host(&host, port, allow_private)?;

        let client = ClientBuilder::new()
            .timeout(request.timeout)
            .redirect(Policy::none())
            .resolve(&host, resolved_addr)
            .build()
            .map_err(|e| format!("failed to create HTTP client: {e}"))?;

        let mut outbound = client.request(request.method.clone(), current_url.as_str());

        // Only attach user headers and body on the initial request.
        if remaining_redirects == HTTP_MAX_REDIRECTS {
            for (name, value) in &request.headers {
                outbound = outbound.header(name, value);
            }
            if let Some(ref payload) = request.body {
                outbound = outbound.body(payload.clone());
            }
        }

        let response = outbound
            .send()
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        if response.status().is_redirection() {
            if remaining_redirects == 0 {
                return Err("too many HTTP redirects".to_string());
            }
            remaining_redirects -= 1;

            let location = response
                .headers()
                .get("location")
                .ok_or_else(|| "redirect response missing Location header".to_string())?
                .to_str()
                .map_err(|e| format!("invalid Location header: {e}"))?;

            let redirect_url = current_url
                .join(location)
                .map_err(|e| format!("invalid redirect URL: {e}"))?;

            // Validate the redirect target with the same URL-level checks.
            normalize_request_url(redirect_url.as_str(), allow_private)?;
            current_url = redirect_url;
            continue;
        }

        let status = i32::from(response.status().as_u16());
        let mut response_headers = ResponseHeaders::new();
        for (name, value) in response.headers() {
            response_headers
                .entry(name.as_str().to_string())
                .or_default()
                .push(response_header_value(value));
        }

        let body = read_response_body_with_limit(response)?;
        return Ok((status, response_headers, body));
    }
}

use ssrf::{normalize_request_url, resolve_and_validate_host};

fn read_response_body_with_limit(response: reqwest::blocking::Response) -> Result<String, String> {
    if let Some(content_length) = response.content_length()
        && content_length > HTTP_MAX_RESPONSE_BODY_BYTES as u64
    {
        return Err(format!(
            "HTTP response body exceeds {} bytes",
            HTTP_MAX_RESPONSE_BODY_BYTES
        ));
    }

    let mut body_buffer = Vec::new();
    response
        .take((HTTP_MAX_RESPONSE_BODY_BYTES as u64) + 1)
        .read_to_end(&mut body_buffer)
        .map_err(|err| format!("failed to read HTTP response body: {err}"))?;

    if body_buffer.len() > HTTP_MAX_RESPONSE_BODY_BYTES {
        return Err(format!(
            "HTTP response body exceeds {} bytes",
            HTTP_MAX_RESPONSE_BODY_BYTES
        ));
    }

    Ok(String::from_utf8_lossy(&body_buffer).into_owned())
}

fn build_response_object(
    context: &mut Context,
    status: i32,
    headers: ResponseHeaders,
    body: String,
) -> Result<JsObject, String> {
    let headers = serde_json::to_value(headers)
        .map_err(|error| format!("failed to serialize response headers: {error}"))?;
    let headers = json_to_js_value(&headers, context)?;

    let mut response = ObjectInitializer::new(context);
    response.property(js_string!("status"), status, Attribute::all());
    response.property(js_string!("headers"), headers, Attribute::all());
    response.property(
        js_string!(RESPONSE_BODY_PROPERTY),
        JsValue::from(js_string!(body.as_str())),
        Attribute::default(),
    );
    response.function(
        NativeFunction::from_fn_ptr(response_text),
        js_string!("text"),
        0,
    );
    response.function(
        NativeFunction::from_fn_ptr(response_json),
        js_string!("json"),
        0,
    );

    Ok(response.build())
}

fn response_text(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_body(this, context).map_err(http_response_error("text"))?;
    // Response body is already buffered from the fetch. We wrap it in a resolved promise
    // to match the Web API contract where .text() and .json() return promises.
    Ok(resolve_promise(context, JsValue::from(js_string!(body.as_str()))).into())
}

fn response_json(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let body = response_body(this, context).map_err(http_response_error("json"))?;
    let parsed = serde_json::from_str::<Value>(&body)
        .map_err(|error| js_error("json", format!("response body is not valid JSON: {error}")))?;
    let value = json_to_js_value(&parsed, context)
        .map_err(|error| js_error("json", format!("failed to convert JSON response: {error}")))?;
    // Response body is already buffered from the fetch. We wrap it in a resolved promise
    // to match the Web API contract where .text() and .json() return promises.
    Ok(resolve_promise(context, value).into())
}

fn response_body(this: &JsValue, context: &mut Context) -> Result<String, String> {
    let object = this
        .as_object()
        .ok_or_else(|| "response object is not available".to_string())?;
    let body = object
        .get(js_string!(RESPONSE_BODY_PROPERTY), context)
        .map_err(format_js_error)?;
    body.to_string(context)
        .map(|value| value.to_std_string_escaped())
        .map_err(format_js_error)
}

fn http_response_error(function: &'static str) -> impl Fn(String) -> JsError + Copy {
    move |message| js_error(function, message)
}

fn parse_fetch_request(
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

fn parse_fetch_request_json(
    input: Value,
    options: Option<Value>,
    allow_private_networks: bool,
) -> Result<HttpRequest, String> {
    match input {
        Value::String(url) => parse_legacy_fetch_request(url, options.as_ref(), allow_private_networks),
        Value::Object(object) => parse_request_object_fetch_request(object, options.as_ref(), allow_private_networks),
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

fn js_error(function: &'static str, message: impl Into<String>) -> JsError {
    super::js_error("volt:http", function, message)
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

fn parse_headers(options: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
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

fn parse_body(options: Option<&Value>) -> Result<Option<String>, String> {
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

fn allow_private_networks_for_tests() -> bool {
    cfg!(test)
}

fn response_header_value(value: &reqwest::header::HeaderValue) -> String {
    match value.to_str() {
        Ok(text) => text.to_string(),
        Err(_) => String::from_utf8_lossy(value.as_bytes()).into_owned(),
    }
}

pub fn build_module(context: &mut Context) -> Module {
    let fetch = fetch.into_js_function_copied(context);
    let exports = vec![("fetch", fetch)];
    native_function_module(context, exports)
}

#[cfg(test)]
mod tests;
