use super::*;

#[test]
fn test_update_config_serde() {
    let config = UpdateConfig {
        endpoint: "https://example.com/updates".to_string(),
        public_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
        current_version: "1.0.0".to_string(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let restored: UpdateConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.endpoint, "https://example.com/updates");
    assert_eq!(restored.current_version, "1.0.0");
}

#[test]
fn test_update_info_serde() {
    let info = UpdateInfo {
        version: "2.0.0".to_string(),
        url: "https://example.com/v2.0.0/app".to_string(),
        signature: "base64sig==".to_string(),
        sha256: "abcdef1234567890".to_string(),
        target: "linux-x64".to_string(),
    };
    let json = serde_json::to_string(&info).unwrap();
    let restored: UpdateInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.version, "2.0.0");
    assert_eq!(restored.sha256, "abcdef1234567890");
}

#[test]
fn test_update_error_display_all_variants() {
    let e = UpdateError::CheckFailed("timeout".into());
    assert!(e.to_string().contains("timeout"));

    let e = UpdateError::DownloadFailed("404".into());
    assert!(e.to_string().contains("404"));

    let e = UpdateError::SignatureInvalid("bad sig".into());
    assert!(e.to_string().contains("bad sig"));

    let e = UpdateError::ChecksumMismatch {
        expected: "aaa".into(),
        actual: "bbb".into(),
    };
    let msg = e.to_string();
    assert!(msg.contains("aaa"));
    assert!(msg.contains("bbb"));

    let e = UpdateError::DowngradePrevented {
        current: "2.0.0".into(),
        offered: "1.0.0".into(),
    };
    let msg = e.to_string();
    assert!(msg.contains("2.0.0"));
    assert!(msg.contains("1.0.0"));

    let e = UpdateError::InsecureUrl("http://evil.com".into());
    assert!(e.to_string().contains("http://evil.com"));

    let e = UpdateError::InvalidPublicKey("wrong format".into());
    assert!(e.to_string().contains("wrong format"));

    let e = UpdateError::ApplyFailed("permission denied".into());
    assert!(e.to_string().contains("permission denied"));
}
