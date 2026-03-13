use std::borrow::Cow;

use wry::http::{Response, StatusCode, header};

use crate::security;

use super::AssetBundle;

/// Determine MIME type from a file path extension.
pub fn mime_type_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "wasm" => "application/wasm",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "audio/ogg",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// Build an HTTP response serving an asset from the bundle with proper security headers.
pub fn serve_asset(bundle: &AssetBundle, request_path: &str) -> Response<Cow<'static, [u8]>> {
    // Normalize path: strip leading slash, default to index.html.
    let path = request_path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let normalized = path.replace('\\', "/");

    // Security: reject path traversal attempts by path segment.
    if normalized
        .split('/')
        .any(|segment| segment == ".." || segment == ".")
    {
        return error_response(StatusCode::FORBIDDEN, "Forbidden");
    }

    match bundle.get(&normalized) {
        Some(data) => {
            let mime = mime_type_for_path(&normalized);
            let csp = security::production_csp();

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .header("Content-Security-Policy", csp)
                .header("Cross-Origin-Opener-Policy", "same-origin")
                .header("X-Content-Type-Options", "nosniff")
                // Required for WebView2: Vite emits <script crossorigin> which forces
                // CORS mode even for same-origin requests on custom protocols.
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "volt://localhost")
                .body(Cow::Owned(data.to_vec()))
                .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "Error"))
        }
        None => error_response(StatusCode::NOT_FOUND, "Not Found"),
    }
}

/// Build a simple error HTTP response.
fn error_response(status: StatusCode, message: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Cow::Owned(message.as_bytes().to_vec()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Cow::Owned(b"Internal Server Error".to_vec()))
                .expect("building fallback response should not fail")
        })
}
