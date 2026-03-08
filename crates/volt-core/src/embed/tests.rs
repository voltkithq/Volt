use super::*;
use std::path::Path;

#[cfg(unix)]
fn create_file_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_file_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(src, dst)
}

#[cfg(unix)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(src, dst)
}

#[cfg(windows)]
fn create_dir_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(src, dst)
}

#[test]
fn test_mime_types() {
    assert_eq!(mime_type_for_path("index.html"), "text/html; charset=utf-8");
    assert_eq!(
        mime_type_for_path("assets/main.js"),
        "application/javascript; charset=utf-8"
    );
    assert_eq!(mime_type_for_path("style.css"), "text/css; charset=utf-8");
    assert_eq!(mime_type_for_path("image.png"), "image/png");
    assert_eq!(mime_type_for_path("font.woff2"), "font/woff2");
    assert_eq!(
        mime_type_for_path("unknown.xyz"),
        "application/octet-stream"
    );
}

#[test]
fn test_bundle_serialize_roundtrip() {
    let mut bundle = AssetBundle::new();
    bundle.insert("index.html".to_string(), b"<html>hello</html>".to_vec());
    bundle.insert("assets/main.js".to_string(), b"console.log('ok')".to_vec());

    let bytes = bundle.to_bytes().unwrap();
    let restored = AssetBundle::from_bytes(&bytes).unwrap();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored.get("index.html").unwrap(), b"<html>hello</html>");
    assert_eq!(
        restored.get("assets/main.js").unwrap(),
        b"console.log('ok')"
    );
}

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
fn test_bundle_from_directory() {
    let temp = std::env::temp_dir().join("volt_test_embed");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(temp.join("assets")).unwrap();
    std::fs::write(temp.join("index.html"), b"<html>test</html>").unwrap();
    std::fs::write(temp.join("assets/main.js"), b"console.log('hi')").unwrap();

    let bundle = AssetBundle::from_directory(&temp).unwrap();
    assert_eq!(bundle.len(), 2);
    assert!(bundle.get("index.html").is_some());
    assert!(bundle.get("assets/main.js").is_some());

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_collect_files_rejects_excessive_recursion_depth() {
    let temp = std::env::temp_dir().join("volt_test_embed_depth");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();
    std::fs::write(temp.join("index.html"), b"<html>depth</html>").unwrap();

    let mut assets = std::collections::HashMap::new();
    let mut visited = std::collections::HashSet::new();
    let err = super::fs::collect_files(
        &temp,
        &temp,
        &mut assets,
        &mut visited,
        super::fs::MAX_ASSET_RECURSION_DEPTH + 1,
    )
    .unwrap_err();
    assert!(err.to_string().contains("recursion depth exceeds limit"));

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_bundle_empty() {
    let bundle = AssetBundle::new();
    assert!(bundle.is_empty());
    assert_eq!(bundle.len(), 0);
    assert!(bundle.get("anything").is_none());
}

#[test]
fn test_bundle_from_directory_rejects_symlink_to_outside_file() {
    let root = std::env::temp_dir().join("volt_test_embed_symlink_outside_file");
    let outside_file = std::env::temp_dir().join("volt_test_embed_symlink_outside_file_secret.txt");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&outside_file);

    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join("index.html"), b"<html>safe</html>").unwrap();
    std::fs::write(&outside_file, b"secret").unwrap();

    if create_file_symlink(&outside_file, &root.join("assets/leak.txt")).is_err() {
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&outside_file);
        return;
    }

    let err = AssetBundle::from_directory(&root).unwrap_err();
    assert!(err.to_string().contains("outside asset root"));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&outside_file);
}

#[test]
fn test_bundle_from_directory_rejects_symlink_to_outside_directory() {
    let root = std::env::temp_dir().join("volt_test_embed_symlink_outside_dir");
    let outside_dir = std::env::temp_dir().join("volt_test_embed_symlink_outside_dir_secret");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside_dir);

    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();
    std::fs::write(outside_dir.join("secret.txt"), b"secret").unwrap();

    if create_dir_symlink(&outside_dir, &root.join("linked")).is_err() {
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside_dir);
        return;
    }

    let err = AssetBundle::from_directory(&root).unwrap_err();
    assert!(err.to_string().contains("outside asset root"));

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside_dir);
}

#[test]
fn test_bundle_from_directory_preserves_symlink_logical_key_for_in_root_target() {
    let root = std::env::temp_dir().join("volt_test_embed_symlink_in_root_key");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::create_dir_all(root.join("shared")).unwrap();
    std::fs::write(root.join("shared/main.js"), b"console.log('shared')").unwrap();

    if create_file_symlink(&root.join("shared/main.js"), &root.join("assets/app.js")).is_err() {
        let _ = std::fs::remove_dir_all(&root);
        return;
    }

    let bundle = AssetBundle::from_directory(&root).unwrap();
    assert!(bundle.get("assets/app.js").is_some());
    assert!(bundle.get("shared/main.js").is_some());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_bundle_default() {
    let bundle = AssetBundle::default();
    assert!(bundle.is_empty());
}

#[test]
fn test_bundle_insert_and_get() {
    let mut bundle = AssetBundle::new();
    bundle.insert("test.txt".to_string(), b"hello".to_vec());
    assert_eq!(bundle.len(), 1);
    assert!(!bundle.is_empty());
    assert_eq!(bundle.get("test.txt").unwrap(), b"hello");
    assert!(bundle.get("nonexistent").is_none());
}

#[test]
fn test_bundle_from_bytes_empty() {
    let result = AssetBundle::from_bytes(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("too short"));
}

#[test]
fn test_bundle_from_bytes_truncated() {
    let data = 1u32.to_le_bytes();
    let result = AssetBundle::from_bytes(&data);
    assert!(result.is_err());
}

#[test]
fn test_bundle_from_bytes_zero_entries() {
    let data = 0u32.to_le_bytes();
    let bundle = AssetBundle::from_bytes(&data).unwrap();
    assert!(bundle.is_empty());
}

#[test]
fn test_mime_type_uppercase_extension() {
    assert_eq!(mime_type_for_path("FILE.HTML"), "text/html; charset=utf-8");
    assert_eq!(mime_type_for_path("image.PNG"), "image/png");
    assert_eq!(mime_type_for_path("style.CSS"), "text/css; charset=utf-8");
}

#[test]
fn test_mime_type_no_extension() {
    assert_eq!(mime_type_for_path("Makefile"), "application/octet-stream");
    assert_eq!(mime_type_for_path("LICENSE"), "application/octet-stream");
}

#[test]
fn test_mime_type_all_types() {
    assert!(mime_type_for_path("f.html").starts_with("text/html"));
    assert!(mime_type_for_path("f.htm").starts_with("text/html"));
    assert!(mime_type_for_path("f.js").starts_with("application/javascript"));
    assert!(mime_type_for_path("f.mjs").starts_with("application/javascript"));
    assert!(mime_type_for_path("f.css").starts_with("text/css"));
    assert!(mime_type_for_path("f.json").starts_with("application/json"));
    assert_eq!(mime_type_for_path("f.png"), "image/png");
    assert_eq!(mime_type_for_path("f.jpg"), "image/jpeg");
    assert_eq!(mime_type_for_path("f.jpeg"), "image/jpeg");
    assert_eq!(mime_type_for_path("f.gif"), "image/gif");
    assert_eq!(mime_type_for_path("f.svg"), "image/svg+xml");
    assert_eq!(mime_type_for_path("f.ico"), "image/x-icon");
    assert_eq!(mime_type_for_path("f.webp"), "image/webp");
    assert_eq!(mime_type_for_path("f.woff"), "font/woff");
    assert_eq!(mime_type_for_path("f.woff2"), "font/woff2");
    assert_eq!(mime_type_for_path("f.ttf"), "font/ttf");
    assert_eq!(mime_type_for_path("f.otf"), "font/otf");
    assert_eq!(mime_type_for_path("f.eot"), "application/vnd.ms-fontobject");
    assert_eq!(mime_type_for_path("f.wasm"), "application/wasm");
    assert_eq!(mime_type_for_path("f.mp3"), "audio/mpeg");
    assert_eq!(mime_type_for_path("f.mp4"), "video/mp4");
    assert_eq!(mime_type_for_path("f.webm"), "video/webm");
    assert_eq!(mime_type_for_path("f.ogg"), "audio/ogg");
    assert!(mime_type_for_path("f.txt").starts_with("text/plain"));
    assert!(mime_type_for_path("f.xml").starts_with("application/xml"));
    assert_eq!(mime_type_for_path("f.pdf"), "application/pdf");
    assert_eq!(mime_type_for_path("f.zip"), "application/zip");
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
