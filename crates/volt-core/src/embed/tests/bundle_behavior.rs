use super::*;

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
