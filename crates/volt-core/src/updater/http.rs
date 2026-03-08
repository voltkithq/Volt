use reqwest::blocking::{Client, Response};
use reqwest::header::LOCATION;
use std::io::Read;
use std::time::Duration;
use url::Url;

use super::util::validate_url_security;
use super::verification::UpdateError;

const UPDATE_HTTP_TIMEOUT: Duration = Duration::from_secs(15);
const UPDATE_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const UPDATE_HTTP_MAX_REDIRECTS: usize = 8;
pub(super) const UPDATE_CHECK_RESPONSE_MAX_BYTES: usize = 512 * 1024;
pub(super) const UPDATE_DOWNLOAD_MAX_BYTES: usize = 256 * 1024 * 1024;

pub(super) fn build_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(UPDATE_HTTP_TIMEOUT)
        .connect_timeout(UPDATE_HTTP_CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

pub(super) fn resolve_redirect_url(current: &Url, location: &str) -> Result<Url, UpdateError> {
    let next = current
        .join(location)
        .map_err(|e| UpdateError::InsecureUrl(format!("invalid redirect URL: {e}")))?;
    validate_url_security(next.as_str())?;
    Ok(next)
}

pub(super) fn fetch_with_validated_redirects(
    client: &Client,
    url: &str,
    context: &str,
) -> Result<Response, String> {
    let mut current = Url::parse(url).map_err(|e| format!("{context}: invalid URL: {e}"))?;

    for _ in 0..=UPDATE_HTTP_MAX_REDIRECTS {
        validate_url_security(current.as_str()).map_err(|e| format!("{context}: {e}"))?;

        let response = client
            .get(current.as_str())
            .send()
            .map_err(|e| format!("{context}: HTTP request failed: {e}"))?;

        if !response.status().is_redirection() {
            return Ok(response);
        }

        let location = response
            .headers()
            .get(LOCATION)
            .ok_or_else(|| format!("{context}: redirect response missing Location header"))?;
        let location_str = location
            .to_str()
            .map_err(|e| format!("{context}: invalid redirect location header: {e}"))?;
        let next =
            resolve_redirect_url(&current, location_str).map_err(|e| format!("{context}: {e}"))?;
        current = next;
    }

    Err(format!(
        "{context}: too many redirects (max {UPDATE_HTTP_MAX_REDIRECTS})"
    ))
}

pub(super) fn read_limited_bytes<R: Read>(
    reader: &mut R,
    max_bytes: usize,
    context: &str,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    let mut chunk = [0_u8; 16 * 1024];
    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|e| format!("{context}: failed to read response body: {e}"))?;
        if read == 0 {
            break;
        }
        if data.len().saturating_add(read) > max_bytes {
            return Err(format!(
                "{context}: response body exceeded size limit ({max_bytes} bytes)"
            ));
        }
        data.extend_from_slice(&chunk[..read]);
    }
    Ok(data)
}

pub(super) fn read_response_body_limited(
    mut response: Response,
    max_bytes: usize,
    context: &str,
) -> Result<Vec<u8>, String> {
    read_limited_bytes(&mut response, max_bytes, context)
}
