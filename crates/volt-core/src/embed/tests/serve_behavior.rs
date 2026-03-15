use super::*;

#[test]
fn test_serve_asset_found() {
    let mut bundle = AssetBundle::new();
    bundle.insert("index.html".to_string(), b"<html>test</html>".to_vec());

    let response = serve_asset(&bundle, "/");
    assert_eq!(response.status(), wry::http::StatusCode::OK);
    assert_eq!(response.body().as_ref(), b"<html>test</html>");

    let response = serve_asset(&bundle, "/index.html");
    assert_eq!(response.status(), wry::http::StatusCode::OK);
}

#[test]
fn test_serve_asset_not_found() {
    let bundle = AssetBundle::new();
    let response = serve_asset(&bundle, "/missing.html");
    assert_eq!(response.status(), wry::http::StatusCode::NOT_FOUND);
}

#[test]
fn test_serve_asset_traversal_blocked() {
    let mut bundle = AssetBundle::new();
    bundle.insert("secret.txt".to_string(), b"secret".to_vec());

    let response = serve_asset(&bundle, "/../etc/passwd");
    assert_eq!(response.status(), wry::http::StatusCode::FORBIDDEN);

    let response = serve_asset(&bundle, "/..\\windows\\system32");
    assert_eq!(response.status(), wry::http::StatusCode::FORBIDDEN);
}

#[test]
fn test_serve_asset_has_security_headers() {
    let mut bundle = AssetBundle::new();
    bundle.insert("index.html".to_string(), b"<html></html>".to_vec());

    let response = serve_asset(&bundle, "/");
    let headers = response.headers();

    assert!(headers.contains_key("Content-Security-Policy"));
    assert!(headers.contains_key("Cross-Origin-Opener-Policy"));
    assert!(headers.contains_key("X-Content-Type-Options"));
    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
    assert_eq!(
        headers.get("Cross-Origin-Opener-Policy").unwrap(),
        "same-origin"
    );
}

#[test]
fn test_serve_asset_correct_content_type() {
    let mut bundle = AssetBundle::new();
    bundle.insert("app.js".to_string(), b"var x = 1;".to_vec());

    let response = serve_asset(&bundle, "/app.js");
    assert_eq!(response.status(), wry::http::StatusCode::OK);
    let content_type = response
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("javascript"));
}

#[test]
fn test_serve_asset_backslash_traversal() {
    let bundle = AssetBundle::new();
    let response = serve_asset(&bundle, "/foo\\..\\etc\\passwd");
    assert_eq!(response.status(), wry::http::StatusCode::FORBIDDEN);
}
