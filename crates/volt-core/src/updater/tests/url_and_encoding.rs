use super::*;
use std::io::Cursor;
use url::Url;

#[test]
fn test_hex_encode() {
    assert_eq!(hex_encode(&[0xab, 0xcd, 0xef]), "abcdef");
    assert_eq!(hex_encode(&[0x00, 0xff]), "00ff");
}

#[test]
fn test_validate_url_https() {
    assert!(validate_url_security("https://example.com/updates").is_ok());
}

#[test]
fn test_validate_url_http_rejected() {
    assert!(validate_url_security("http://example.com/updates").is_err());
}

#[test]
fn test_validate_url_localhost_http_allowed() {
    assert!(validate_url_security("http://localhost:8080/updates").is_ok());
    assert!(validate_url_security("http://127.0.0.1:8080/updates").is_ok());
}

#[test]
fn test_build_update_check_url_encodes_query_values() {
    let url = build_update_check_url(
        "https://updates.example.com/check",
        "1.0.0 beta+1",
        "windows x64",
    )
    .expect("build update URL");
    assert!(url.contains("current_version=1.0.0+beta%2B1"));
    assert!(url.contains("target=windows+x64"));
}

#[test]
fn test_current_target() {
    let target = current_target();
    assert!(!target.is_empty());
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    assert_eq!(target, "windows-x64");
}

#[test]
fn test_validate_url_127001_localhost_allowed() {
    assert!(validate_url_security("http://127.0.0.1:3000/updates").is_ok());
    assert!(validate_url_security("http://127.0.0.1/updates").is_ok());
}

#[test]
fn test_validate_url_ipv6_localhost_allowed() {
    assert!(validate_url_security("http://[::1]:8080/updates").is_ok());
    assert!(validate_url_security("http://[::1]/updates").is_ok());
}

#[test]
fn test_validate_url_ftp_rejected() {
    assert!(validate_url_security("ftp://example.com/updates").is_err());
}

#[test]
fn test_validate_url_empty_string() {
    assert!(validate_url_security("").is_err());
}

#[test]
fn test_validate_url_no_scheme() {
    assert!(validate_url_security("example.com/updates").is_err());
}

#[test]
fn test_resolve_redirect_url_rejects_insecure_target() {
    let current = Url::parse("https://updates.example.com/check").unwrap();
    let result = resolve_redirect_url(&current, "http://evil.example.com/update");
    assert!(result.is_err());
}

#[test]
fn test_resolve_redirect_url_allows_safe_relative_target() {
    let current = Url::parse("https://updates.example.com/check").unwrap();
    let redirected = resolve_redirect_url(&current, "/v2/check").unwrap();
    assert_eq!(redirected.as_str(), "https://updates.example.com/v2/check");
}

#[test]
fn test_read_limited_bytes_enforces_limit() {
    let mut over_limit = Cursor::new(vec![1_u8; 8]);
    let err = read_limited_bytes(&mut over_limit, 4, "download").unwrap_err();
    assert!(err.contains("exceeded size limit"));

    let mut in_limit = Cursor::new(vec![1_u8, 2_u8, 3_u8]);
    let body = read_limited_bytes(&mut in_limit, 4, "download").unwrap();
    assert_eq!(body, vec![1_u8, 2_u8, 3_u8]);
}

#[test]
fn test_hex_encode_empty() {
    assert_eq!(hex_encode(&[]), "");
}

#[test]
fn test_hex_encode_single_byte() {
    assert_eq!(hex_encode(&[0x0a]), "0a");
    assert_eq!(hex_encode(&[0xff]), "ff");
    assert_eq!(hex_encode(&[0x00]), "00");
}
