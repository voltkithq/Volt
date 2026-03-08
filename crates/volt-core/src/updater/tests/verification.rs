use super::*;

#[test]
fn test_canonical_update_metadata_payload_is_stable() {
    let info = UpdateInfo {
        version: "1.2.3".to_string(),
        url: "https://updates.example.com/app.exe".to_string(),
        signature: String::new(),
        sha256: "AB".repeat(32),
    };

    let payload = super::super::verification::canonical_update_metadata_payload(&info);
    assert_eq!(
        payload,
        b"volt-update-v1\01.2.3\0https://updates.example.com/app.exe\0abababababababababababababababababababababababababababababababab".to_vec()
    );
}

#[test]
fn test_metadata_signature_verification_accepts_signed_metadata() {
    let info = signed_update_info(
        "2.0.0",
        "https://updates.example.com/app.exe",
        &"ab".repeat(32),
    );
    let config = UpdateConfig {
        endpoint: "https://updates.example.com/check".to_string(),
        public_key: test_public_key_b64(),
        current_version: "1.0.0".to_string(),
    };

    assert!(super::super::verification::verify_update_metadata(&config, &info).is_ok());
}

#[test]
fn test_metadata_signature_verification_rejects_version_tampering() {
    let mut info = signed_update_info(
        "1.0.1",
        "https://updates.example.com/app.exe",
        &"ab".repeat(32),
    );
    info.version = "2.0.0".to_string();
    let config = UpdateConfig {
        endpoint: "https://updates.example.com/check".to_string(),
        public_key: test_public_key_b64(),
        current_version: "1.0.0".to_string(),
    };

    assert!(matches!(
        super::super::verification::verify_update_metadata(&config, &info),
        Err(UpdateError::SignatureInvalid(_))
    ));
}

#[test]
fn test_invalid_public_key() {
    assert!(decode_public_key("not-valid-base64!!!").is_err());
    assert!(decode_public_key("AAAA").is_err());
}
