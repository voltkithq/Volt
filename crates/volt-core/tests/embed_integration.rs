//! Integration tests for the asset embedding and serving pipeline.
//! Tests the full cycle: create bundle → serialize → deserialize → serve.

use volt_core::embed::{AssetBundle, serve_asset};
use wry::http::StatusCode;

#[test]
fn bundle_create_serialize_deserialize_serve() {
    // 1. Create a bundle with multiple file types
    let mut bundle = AssetBundle::new();
    bundle.insert(
        "index.html".to_string(),
        b"<!DOCTYPE html><html><body>Hello Volt</body></html>".to_vec(),
    );
    bundle.insert(
        "assets/app.js".to_string(),
        b"console.log('Volt app');".to_vec(),
    );
    bundle.insert(
        "assets/style.css".to_string(),
        b"body { margin: 0; }".to_vec(),
    );
    bundle.insert(
        "assets/icon.png".to_string(),
        vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes
    );

    assert_eq!(bundle.len(), 4);

    // 2. Serialize to binary format
    let bytes = bundle.to_bytes().unwrap();
    assert!(!bytes.is_empty());

    // 3. Deserialize back
    let restored = AssetBundle::from_bytes(&bytes).unwrap();
    assert_eq!(restored.len(), 4);

    // 4. Verify all files are intact
    assert_eq!(
        restored.get("index.html").unwrap(),
        b"<!DOCTYPE html><html><body>Hello Volt</body></html>"
    );
    assert_eq!(
        restored.get("assets/app.js").unwrap(),
        b"console.log('Volt app');"
    );
    assert_eq!(
        restored.get("assets/style.css").unwrap(),
        b"body { margin: 0; }"
    );
    assert_eq!(
        restored.get("assets/icon.png").unwrap(),
        &[0x89, 0x50, 0x4E, 0x47]
    );

    // 5. Serve assets and verify HTTP responses
    let resp = serve_asset(&restored, "/");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/html"));

    let resp = serve_asset(&restored, "/assets/app.js");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("javascript"));

    let resp = serve_asset(&restored, "/assets/style.css");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/css"));

    let resp = serve_asset(&restored, "/assets/icon.png");
    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(ct, "image/png");

    // 6. Non-existent asset returns 404
    let resp = serve_asset(&restored, "/missing.html");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 7. Traversal attempts return 403
    let resp = serve_asset(&restored, "/../secret");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[test]
fn bundle_from_directory_roundtrip() {
    // Create a temporary directory with files
    let temp = std::env::temp_dir().join("volt_integration_embed");
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(temp.join("assets")).unwrap();
    std::fs::write(temp.join("index.html"), "<html>integration test</html>").unwrap();
    std::fs::write(temp.join("assets/main.js"), "export default {};").unwrap();

    // Build from directory
    let bundle = AssetBundle::from_directory(&temp).unwrap();
    assert_eq!(bundle.len(), 2);
    assert!(bundle.get("index.html").is_some());
    assert!(bundle.get("assets/main.js").is_some());

    // Serialize and restore
    let bytes = bundle.to_bytes().unwrap();
    let restored = AssetBundle::from_bytes(&bytes).unwrap();
    assert_eq!(restored.len(), 2);
    assert_eq!(
        std::str::from_utf8(restored.get("index.html").unwrap()).unwrap(),
        "<html>integration test</html>"
    );

    // Serve from restored bundle
    let resp = serve_asset(&restored, "/");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.body().as_ref(), b"<html>integration test</html>");

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp);
}
