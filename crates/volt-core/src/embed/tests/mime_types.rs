use super::*;

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
