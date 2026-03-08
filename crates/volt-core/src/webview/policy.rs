use url::Url;

use super::{WebViewConfig, WebViewSource};

const VOLT_PROTOCOL_ALIAS_ORIGIN: &str = "https://volt.localhost";

pub(super) fn navigation_origins_for(config: &WebViewConfig) -> Vec<String> {
    let mut origins = config.allowed_origins.clone();

    if let WebViewSource::Url(url) = &config.source
        && let Ok(parsed) = Url::parse(url)
    {
        match parsed.scheme() {
            "http" | "https" => origins.push(parsed.origin().ascii_serialization()),
            "volt" => origins.push(VOLT_PROTOCOL_ALIAS_ORIGIN.to_string()),
            _ => {}
        }
    }

    origins
}

/// Check if a URL's origin is allowed for navigation.
pub(super) fn is_origin_allowed(url_str: &str, allowed_origins: &[String]) -> bool {
    if url_str == "about:blank" {
        return true;
    }

    if url_str.starts_with("data:") {
        return is_safe_data_url(url_str);
    }

    if url_str.starts_with("volt://") {
        return true;
    }

    let parsed = match Url::parse(url_str) {
        Ok(url) => url,
        Err(_) => return false,
    };

    if !is_allowed_navigation_scheme(parsed.scheme()) {
        return false;
    }

    let host = parsed.host_str().unwrap_or("");
    let normalized_host = host.trim_matches(['[', ']']).to_ascii_lowercase();

    for origin in allowed_origins {
        if let Ok(allowed) = Url::parse(origin) {
            if parsed.scheme() == allowed.scheme()
                && parsed.host_str() == allowed.host_str()
                && parsed.port() == allowed.port()
            {
                return true;
            }
        } else if normalized_host == origin.to_ascii_lowercase()
            && is_allowed_navigation_scheme(parsed.scheme())
        {
            // Bare hostname allowlist entries are only valid for safe web schemes.
            return true;
        }
    }

    false
}

fn is_allowed_navigation_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https")
}

fn is_safe_data_url(url_str: &str) -> bool {
    let Some(rest) = url_str.strip_prefix("data:") else {
        return false;
    };

    let media = rest.split(',').next().unwrap_or_default().trim();
    let media_type = media.split(';').next().unwrap_or_default().trim();
    let media_type = if media_type.is_empty() {
        "text/plain"
    } else {
        media_type
    };
    let lowered = media_type.to_ascii_lowercase();
    lowered.starts_with("image/") || lowered.starts_with("font/")
}
