use super::super::policy::is_origin_allowed;

#[test]
fn test_invalid_url_blocked() {
    assert!(!is_origin_allowed("not-a-url", &[]));
}

#[test]
fn test_localhost_unsafe_schemes_blocked() {
    assert!(!is_origin_allowed("file://localhost/etc/passwd", &[]));
    assert!(!is_origin_allowed("ftp://127.0.0.1/resource", &[]));
}

#[test]
fn test_bare_hostname_unsafe_scheme_blocked() {
    let allowed = vec!["example.com".to_string()];
    assert!(!is_origin_allowed("ftp://example.com/path", &allowed));
    assert!(!is_origin_allowed("file://example.com/path", &allowed));
}

#[test]
fn test_port_mismatch_blocked() {
    let allowed = vec!["https://example.com:443".to_string()];
    assert!(!is_origin_allowed(
        "https://example.com:8443/path",
        &allowed
    ));
}

#[test]
fn test_scheme_mismatch_blocked() {
    let allowed = vec!["https://example.com".to_string()];
    assert!(!is_origin_allowed("http://example.com/path", &allowed));
}
