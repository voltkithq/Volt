use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use sha2::Digest;
use std::io::{Read, Write};
use std::net::TcpListener;

fn metadata_signature(signing_key: &SigningKey, info: &UpdateInfo) -> String {
    base64::engine::general_purpose::STANDARD.encode(
        signing_key
            .sign(&super::super::verification::canonical_update_metadata_payload(info))
            .to_bytes(),
    )
}

fn run_with_payload_server(
    data: &[u8],
    config: &UpdateConfig,
    build_info: impl FnOnce(String) -> UpdateInfo,
) -> Result<Vec<u8>, UpdateError> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind payload listener");
    let addr = listener.local_addr().expect("listener addr");
    let payload = data.to_vec();
    let payload_len = payload.len();
    let url = format!("http://{addr}/artifact");
    let info = build_info(url);
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept payload request");
        let mut request_buf = [0_u8; 2048];
        let _ = stream.read(&mut request_buf);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {payload_len}\r\nConnection: close\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response headers");
        stream.write_all(&payload).expect("write payload");
        stream.flush().expect("flush payload");
    });

    let result = download_and_verify(config, &info);
    server.join().expect("join payload server");
    result
}

#[test]
fn test_signature_validation_regression_matrix() {
    let signing_key = SigningKey::from_bytes(&[9u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let public_key_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());
    let data = b"signed artifact bytes for updater";

    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let sha256_hex = hex_encode(&hash);

    let config = UpdateConfig {
        endpoint: "https://updates.example.com/check".to_string(),
        public_key: public_key_b64,
        current_version: "1.0.0".to_string(),
    };

    let valid = run_with_payload_server(data, &config, |url| {
        let mut info = UpdateInfo {
            version: "2.0.0".to_string(),
            url,
            signature: String::new(),
            sha256: sha256_hex.clone(),
        };
        info.signature = metadata_signature(&signing_key, &info);
        info
    })
    .expect("valid signature should pass");
    assert_eq!(valid, data);

    let bad_signature = download_and_verify(
        &config,
        &UpdateInfo {
            version: "2.0.0".to_string(),
            url: "https://updates.example.com/artifact".to_string(),
            signature: base64::engine::general_purpose::STANDARD.encode([0_u8; 64]),
            sha256: sha256_hex.clone(),
        },
    )
    .expect_err("invalid signature must fail");
    assert!(matches!(bad_signature, UpdateError::SignatureInvalid(_)));

    let bad_checksum = run_with_payload_server(data, &config, |url| {
        let mut info = UpdateInfo {
            version: "2.0.0".to_string(),
            url,
            signature: String::new(),
            sha256: "0".repeat(64),
        };
        info.signature = metadata_signature(&signing_key, &info);
        info
    })
    .expect_err("invalid checksum must fail");
    assert!(matches!(bad_checksum, UpdateError::ChecksumMismatch { .. }));

    let legacy_artifact_signature =
        base64::engine::general_purpose::STANDARD.encode(signing_key.sign(data).to_bytes());
    let legacy = download_and_verify(
        &config,
        &UpdateInfo {
            version: "2.0.0".to_string(),
            url: "https://updates.example.com/artifact".to_string(),
            signature: legacy_artifact_signature,
            sha256: sha256_hex.clone(),
        },
    )
    .expect_err("legacy artifact-only signatures must fail");
    assert!(matches!(legacy, UpdateError::SignatureInvalid(_)));

    let tampered_version_error = download_and_verify(&config, &{
        let mut info = UpdateInfo {
            version: "1.5.0".to_string(),
            url: "https://updates.example.com/artifact".to_string(),
            signature: String::new(),
            sha256: sha256_hex.clone(),
        };
        info.signature = metadata_signature(&signing_key, &info);
        info.version = "2.0.0".to_string();
        info
    })
    .expect_err("tampered version must fail signature verification");
    assert!(matches!(
        tampered_version_error,
        UpdateError::SignatureInvalid(_)
    ));

    let tampered_url_error = download_and_verify(&config, &{
        let mut info = UpdateInfo {
            version: "2.0.0".to_string(),
            url: "https://updates.example.com/artifact".to_string(),
            signature: String::new(),
            sha256: sha256_hex.clone(),
        };
        info.signature = metadata_signature(&signing_key, &info);
        info.url = "https://updates.example.com/artifact?mirror=1".to_string();
        info
    })
    .expect_err("tampered url must fail");
    assert!(matches!(
        tampered_url_error,
        UpdateError::SignatureInvalid(_)
    ));
}
