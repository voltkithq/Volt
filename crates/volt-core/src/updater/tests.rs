use super::http::{read_limited_bytes, resolve_redirect_url};
use super::platform::current_target;
use super::util::{build_update_check_url, hex_encode, validate_url_security};
use super::verification::decode_public_key;
use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use std::io::{Read, Write};
use std::net::TcpListener;

#[path = "tests/check_matrix.rs"]
mod check_matrix;
#[path = "tests/serde_and_errors.rs"]
mod serde_and_errors;
#[path = "tests/signature_matrix.rs"]
mod signature_matrix;
#[path = "tests/url_and_encoding.rs"]
mod url_and_encoding;
#[path = "tests/verification.rs"]
mod verification;

pub(super) fn test_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

pub(super) fn test_public_key_b64() -> String {
    base64::engine::general_purpose::STANDARD.encode(test_signing_key().verifying_key().to_bytes())
}

pub(super) fn sign_update_info(info: &UpdateInfo) -> String {
    let signature = test_signing_key().sign(
        &super::verification::canonical_update_metadata_payload(info),
    );
    base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
}

pub(super) fn signed_update_info(version: &str, url: &str, sha256: &str) -> UpdateInfo {
    let mut info = UpdateInfo {
        version: version.to_string(),
        url: url.to_string(),
        signature: String::new(),
        sha256: sha256.to_string(),
    };
    info.signature = sign_update_info(&info);
    info
}

pub(super) fn check_for_update_against_single_response(
    status_line: &str,
    body: &str,
) -> Result<Option<UpdateInfo>, UpdateError> {
    check_for_update_against_single_response_with_current(status_line, body, "1.0.0")
}

pub(super) fn check_for_update_against_single_response_with_current(
    status_line: &str,
    body: &str,
    current_version: &str,
) -> Result<Option<UpdateInfo>, UpdateError> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("read test listener addr");

    let response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut request_buf = [0_u8; 2048];
        let _ = stream.read(&mut request_buf);
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });

    let config = UpdateConfig {
        endpoint: format!("http://{addr}/updates"),
        public_key: test_public_key_b64(),
        current_version: current_version.to_string(),
    };

    let result = check_for_update(&config);
    server.join().expect("join test server");
    result
}
