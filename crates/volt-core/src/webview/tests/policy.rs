use super::super::policy::{is_origin_allowed, navigation_origins_for};
use super::super::{WebViewConfig, WebViewSource};

#[test]
fn test_source_origin_is_allowlisted_for_navigation() {
    let config = WebViewConfig {
        source: WebViewSource::Url("http://localhost:5173/index.html".to_string()),
        ..WebViewConfig::default()
    };
    let allowed = navigation_origins_for(&config);

    assert!(is_origin_allowed("http://localhost:5173/", &allowed));
    assert!(!is_origin_allowed("http://localhost:3000/", &allowed));
    assert!(!is_origin_allowed("http://127.0.0.1:5173/", &allowed));
}

#[test]
fn test_volt_source_allows_webview2_alias_navigation() {
    let config = WebViewConfig {
        source: WebViewSource::Url("volt://localhost/index.html".to_string()),
        ..WebViewConfig::default()
    };
    let allowed = navigation_origins_for(&config);

    assert!(is_origin_allowed(
        "https://volt.localhost/index.html",
        &allowed
    ));
}

#[test]
fn test_volt_protocol_allowed() {
    assert!(is_origin_allowed("volt://app/index.html", &[]));
}

#[test]
fn test_about_blank_allowed() {
    assert!(is_origin_allowed("about:blank", &[]));
}

#[test]
fn test_external_blocked_by_default() {
    assert!(!is_origin_allowed("https://evil.com", &[]));
    assert!(!is_origin_allowed("http://example.com", &[]));
}

#[test]
fn test_explicit_origin_allowed() {
    let allowed = vec!["https://app.example.com".to_string()];
    assert!(is_origin_allowed("https://app.example.com/app", &allowed));
    assert!(!is_origin_allowed("https://evil.com", &allowed));
}

#[test]
fn test_data_url_allowed() {
    assert!(is_origin_allowed("data:image/png;base64,AAAA", &[]));
    assert!(is_origin_allowed("data:font/woff2;base64,AAAA", &[]));
    assert!(!is_origin_allowed("data:text/html,<h1>hello</h1>", &[]));
    assert!(!is_origin_allowed("data:application/json,{}", &[]));
}

#[test]
fn test_explicit_localhost_ipv6_allowlist_is_respected() {
    let allowed = vec!["http://[::1]:3000".to_string()];
    assert!(is_origin_allowed("http://[::1]:3000/path", &allowed));
}

#[test]
fn test_localhost_navigation_requires_explicit_allowlist() {
    assert!(!is_origin_allowed("http://localhost:5173/", &[]));
    assert!(!is_origin_allowed("http://localhost:3000/", &[]));
    assert!(!is_origin_allowed("http://127.0.0.1:4000/", &[]));
    assert!(!is_origin_allowed("https://volt.localhost:443/", &[]));
}

#[test]
fn test_only_explicit_volt_localhost_alias_is_allowed() {
    assert!(!is_origin_allowed(
        "https://other.localhost/index.html",
        &[]
    ));
}

#[test]
fn test_bare_hostname_in_allowed_origins() {
    let allowed = vec!["example.com".to_string()];
    assert!(is_origin_allowed("https://example.com/path", &allowed));
}
