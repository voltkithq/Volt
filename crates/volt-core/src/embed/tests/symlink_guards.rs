use super::*;

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
