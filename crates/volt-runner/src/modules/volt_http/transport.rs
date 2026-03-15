use std::sync::atomic::{AtomicUsize, Ordering};

use boa_engine::Context;
use boa_engine::object::JsObject;
use reqwest::blocking::ClientBuilder;
use reqwest::redirect::Policy;

use super::allow_private_networks_for_tests;
use super::constants::{
    FetchResult, HTTP_MAX_CONCURRENT_REQUESTS, HTTP_MAX_REDIRECTS, ResponseHeaders,
};
use super::request::HttpRequest;
use super::response::{
    build_response_object, read_response_body_with_limit, response_header_value,
};
use super::ssrf::{normalize_request_url, resolve_and_validate_host};

static HTTP_REQUESTS_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);

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

pub(crate) fn try_acquire_inflight_slot(counter: &AtomicUsize, max: usize) -> bool {
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

pub(super) fn fetch_impl(request: HttpRequest, context: &mut Context) -> Result<JsObject, String> {
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

fn execute_with_dns_pinning(
    request: HttpRequest,
    allow_private: bool,
) -> Result<FetchResult, String> {
    let mut current_url =
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
