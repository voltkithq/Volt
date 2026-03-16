//! Content Security Policy generation for WebView responses.

const VOLT_EMBEDDED_ORIGINS: &str = "volt://localhost http://volt.localhost https://volt.localhost";

/// Default CSP for production builds - strict, no unsafe-eval.
pub fn production_csp() -> String {
    [
        "default-src 'none'",
        &format!("script-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
        &format!("style-src 'self' 'unsafe-inline' {VOLT_EMBEDDED_ORIGINS}"),
        &format!("img-src 'self' data: {VOLT_EMBEDDED_ORIGINS}"),
        &format!("font-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
        &format!("connect-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
    ]
    .join("; ")
}

/// CSP for development builds - allows connections to localhost dev servers.
pub fn development_csp(dev_server_origin: &str) -> String {
    let Some((safe_http_origin, safe_ws_origin)) = sanitize_dev_server_origin(dev_server_origin)
    else {
        return [
            "default-src 'none'",
            &format!("script-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
            &format!("style-src 'self' 'unsafe-inline' {VOLT_EMBEDDED_ORIGINS}"),
            &format!("img-src 'self' data: {VOLT_EMBEDDED_ORIGINS}"),
            &format!("font-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
            &format!("connect-src 'self' {VOLT_EMBEDDED_ORIGINS}"),
        ]
        .join("; ");
    };

    [
        "default-src 'none'",
        &format!("script-src 'self' {VOLT_EMBEDDED_ORIGINS} {safe_http_origin}"),
        &format!("style-src 'self' 'unsafe-inline' {VOLT_EMBEDDED_ORIGINS} {safe_http_origin}"),
        &format!("img-src 'self' data: {VOLT_EMBEDDED_ORIGINS} {safe_http_origin}"),
        &format!("font-src 'self' {VOLT_EMBEDDED_ORIGINS} {safe_http_origin}"),
        &format!("connect-src 'self' {VOLT_EMBEDDED_ORIGINS} {safe_http_origin} {safe_ws_origin}"),
    ]
    .join("; ")
}

fn sanitize_dev_server_origin(dev_server_origin: &str) -> Option<(String, String)> {
    let parsed = url::Url::parse(dev_server_origin).ok()?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    if host.contains(';')
        || host.contains('\n')
        || host.contains('\r')
        || host.chars().any(|c| c.is_ascii_whitespace())
    {
        return None;
    }
    let mut http_origin = format!("{scheme}://{host}");
    if let Some(port) = parsed.port() {
        http_origin.push(':');
        http_origin.push_str(&port.to_string());
    }
    let ws_scheme = if scheme == "https" { "wss" } else { "ws" };
    let ws_origin = http_origin.replacen(scheme, ws_scheme, 1);
    Some((http_origin, ws_origin))
}

/// Validate that a path string does not attempt directory traversal.
pub fn validate_path(path: &str) -> Result<(), String> {
    // Reject null bytes (defense-in-depth — Rust's stdlib also rejects them)
    if path.contains('\0') {
        return Err("Null bytes are not allowed in paths".to_string());
    }

    // Reject absolute paths
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("Absolute paths are not allowed".to_string());
    }

    // Reject Windows drive letters
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err("Absolute paths are not allowed".to_string());
    }

    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    // Check ALL path components (not just the last) for traversal and reserved names
    for component in path.split(['/', '\\']) {
        if component == ".." {
            return Err("Path traversal (..) is not allowed".to_string());
        }

        let base_name = component.split('.').next().unwrap_or("");
        if reserved.iter().any(|r| r.eq_ignore_ascii_case(base_name)) {
            return Err(format!("Reserved device name '{base_name}' is not allowed"));
        }
    }

    Ok(())
}

/// Validate a URL's scheme against the allowed protocol list.
pub fn validate_url_scheme(url: &str) -> Result<(), String> {
    let allowed_schemes = ["http", "https", "mailto"];

    if let Ok(parsed) = url::Url::parse(url) {
        let scheme = parsed.scheme();
        if allowed_schemes.contains(&scheme) {
            return Ok(());
        }
        return Err(format!(
            "Protocol '{scheme}' is not allowed. Only http, https, and mailto are permitted."
        ));
    }

    Err("Invalid URL".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_csp() {
        let csp = production_csp();
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("script-src 'self'"));
        assert!(csp.contains("volt://localhost"));
        assert!(csp.contains("https://volt.localhost"));
        assert!(!csp.contains("unsafe-eval"));
        assert!(!csp.contains("*"));
    }

    #[test]
    fn test_path_traversal_blocked() {
        assert!(validate_path("../../etc/passwd").is_err());
        assert!(validate_path("foo/../../../bar").is_err());
        assert!(validate_path("/etc/passwd").is_err());
        assert!(validate_path("C:\\Windows\\System32").is_err());
    }

    #[test]
    fn test_valid_paths() {
        assert!(validate_path("data/config.json").is_ok());
        assert!(validate_path("images/icon.png").is_ok());
        assert!(validate_path("file.txt").is_ok());
    }

    #[test]
    fn test_reserved_device_names() {
        assert!(validate_path("CON").is_err());
        assert!(validate_path("PRN").is_err());
        assert!(validate_path("NUL").is_err());
        assert!(validate_path("COM1").is_err());
        assert!(validate_path("LPT1").is_err());
        assert!(validate_path("con.txt").is_err());
    }

    #[test]
    fn test_url_scheme_validation() {
        assert!(validate_url_scheme("https://example.com").is_ok());
        assert!(validate_url_scheme("http://localhost:3000").is_ok());
        assert!(validate_url_scheme("mailto:user@example.com").is_ok());

        assert!(validate_url_scheme("file:///etc/passwd").is_err());
        assert!(validate_url_scheme("javascript:alert(1)").is_err());
        assert!(validate_url_scheme("data:text/html,<script>").is_err());
        assert!(validate_url_scheme("vbscript:msgbox").is_err());
    }

    // ── Expanded tests ─────────────────────────────────────────────

    #[test]
    fn test_development_csp_includes_origin() {
        let csp = development_csp("http://localhost:5173");
        assert!(csp.contains("http://localhost:5173"));
        assert!(csp.contains("volt://localhost"));
        assert!(csp.contains("https://volt.localhost"));
        assert!(csp.contains("connect-src"));
        assert!(csp.contains("script-src"));
    }

    #[test]
    fn test_development_csp_includes_websocket() {
        let csp = development_csp("http://localhost:5173");
        assert!(csp.contains("ws://localhost:5173"));
        assert!(
            !csp.contains("ws://http://"),
            "should not have double protocol"
        );
    }

    #[test]
    fn test_development_csp_https_uses_wss() {
        let csp = development_csp("https://localhost:5173");
        assert!(csp.contains("wss://localhost:5173"));
        assert!(
            !csp.contains("ws://https://"),
            "should not have double protocol"
        );
    }

    #[test]
    fn test_development_csp_rejects_invalid_origin_injection() {
        let csp = development_csp("http://localhost:5173;script-src *");
        assert!(!csp.contains("localhost:5173;script-src"));
        assert!(csp.contains("script-src 'self'"));
        assert!(!csp.contains("script-src *"));
    }

    #[test]
    fn test_production_csp_has_only_explicit_localhost_allowances() {
        let csp = production_csp();
        assert!(csp.contains("https://volt.localhost"));
        assert!(!csp.contains("http://localhost"));
        assert!(!csp.contains("https://localhost"));
        assert!(!csp.contains("ws://"));
    }

    #[test]
    fn test_path_empty_string() {
        // Empty string should be valid (it's a relative path with no components)
        assert!(validate_path("").is_ok());
    }

    #[test]
    fn test_path_with_backslash_traversal() {
        assert!(validate_path("foo\\..\\bar").is_err());
        assert!(validate_path("..\\secret").is_err());
    }

    #[test]
    fn test_path_single_dot() {
        // Single dot is a valid relative path (current directory)
        assert!(validate_path("./file.txt").is_ok());
        assert!(validate_path(".").is_ok());
    }

    #[test]
    fn test_path_reserved_case_insensitive() {
        assert!(validate_path("con").is_err());
        assert!(validate_path("Con").is_err());
        assert!(validate_path("CON").is_err());
        assert!(validate_path("prn").is_err());
        assert!(validate_path("nul").is_err());
        assert!(validate_path("com1").is_err());
        assert!(validate_path("lpt1").is_err());
    }

    #[test]
    fn test_path_reserved_with_extension() {
        // "con.txt" should still be blocked (basename before dot is "con")
        assert!(validate_path("con.txt").is_err());
        assert!(validate_path("COM1.log").is_err());
    }

    #[test]
    fn test_path_reserved_in_subdirectory() {
        // The last component's base name is checked
        assert!(validate_path("subdir/CON").is_err());
        assert!(validate_path("a/b/NUL.txt").is_err());
    }

    #[test]
    fn test_path_not_reserved_partial_match() {
        // "CONSOLE" is NOT a reserved name (only exact "CON" matches)
        assert!(validate_path("CONSOLE.log").is_ok());
        assert!(validate_path("PRINTER").is_ok());
        assert!(validate_path("connection.json").is_ok());
    }

    #[test]
    fn test_url_scheme_ftp_blocked() {
        assert!(validate_url_scheme("ftp://example.com/file.txt").is_err());
    }

    #[test]
    fn test_url_scheme_ssh_blocked() {
        assert!(validate_url_scheme("ssh://server.com").is_err());
    }

    #[test]
    fn test_url_scheme_invalid_url() {
        assert!(validate_url_scheme("not a url at all").is_err());
    }
}
