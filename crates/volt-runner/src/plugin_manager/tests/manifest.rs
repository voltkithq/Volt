use std::fs;

use super::super::*;
use super::fs_support::{TempDir, create_dir_symlink, write_manifest};

#[test]
fn parse_plugin_manifest_rejects_hyphenated_ids() {
    let root = TempDir::new("manifest-hyphen");
    let plugin_root = root.join("plugin");
    fs::create_dir_all(plugin_root.join("dist")).expect("plugin dir");
    fs::write(plugin_root.join("dist/plugin.js"), b"export default {};\n").expect("backend");

    let manifest = serde_json::json!({
        "id": "acme.bad-plugin",
        "name": "Bad Plugin",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs"]
    });

    let error = parse_plugin_manifest(
        &serde_json::to_vec(&manifest).expect("manifest json"),
        &plugin_root,
    )
    .expect_err("manifest should be rejected");
    assert!(error.contains("reverse-domain"));
}

#[test]
fn parse_plugin_manifest_rejects_duplicate_capabilities() {
    let root = TempDir::new("manifest-dupes");
    let plugin_root = root.join("plugin");
    fs::create_dir_all(plugin_root.join("dist")).expect("plugin dir");
    fs::write(plugin_root.join("dist/plugin.js"), b"export default {};\n").expect("backend");

    let manifest = serde_json::json!({
        "id": "acme.search",
        "name": "Duplicate Caps",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs", "fs"]
    });

    let error = parse_plugin_manifest(
        &serde_json::to_vec(&manifest).expect("manifest json"),
        &plugin_root,
    )
    .expect_err("manifest should be rejected");
    assert!(error.contains("duplicate capability"));
}

#[test]
fn parse_plugin_manifest_rejects_missing_backend_entry() {
    let root = TempDir::new("manifest-missing-backend");
    let plugin_root = root.join("plugin");
    fs::create_dir_all(&plugin_root).expect("plugin dir");

    let manifest = serde_json::json!({
        "id": "acme.search",
        "name": "Missing Backend",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs"]
    });

    let error = parse_plugin_manifest(
        &serde_json::to_vec(&manifest).expect("manifest json"),
        &plugin_root,
    )
    .expect_err("manifest should be rejected");
    assert!(error.contains("does not exist"));
}

#[test]
fn parse_plugin_manifest_reads_prefetch_on_surfaces() {
    let root = TempDir::new("manifest-prefetch");
    let plugin_root = root.join("plugin");
    fs::create_dir_all(plugin_root.join("dist")).expect("plugin dir");
    fs::write(plugin_root.join("dist/plugin.js"), b"export default {};\n").expect("backend");

    let manifest = serde_json::json!({
        "id": "acme.search",
        "name": "Search",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": ["fs"],
        "prefetchOn": ["search-panel", "file-explorer"]
    });

    let parsed = parse_plugin_manifest(
        &serde_json::to_vec(&manifest).expect("manifest json"),
        &plugin_root,
    )
    .expect("manifest should parse");
    assert_eq!(parsed.prefetch_on, vec!["search-panel", "file-explorer"]);
}

#[test]
fn parse_plugin_route_accepts_valid_routes() {
    let route = parse_plugin_route("plugin:acme.search:ping")
        .expect("route")
        .expect("plugin route");
    assert_eq!(route.plugin_id, "acme.search");
    assert_eq!(route.method, "ping");
}

#[test]
fn parse_plugin_route_rejects_missing_channel() {
    let error = parse_plugin_route("plugin:acme.search").expect_err("invalid route");
    assert!(error.contains("plugin:<plugin-id>:<channel>"));
}

#[test]
fn parse_plugin_route_rejects_empty_plugin_id() {
    let error = parse_plugin_route("plugin::ping").expect_err("invalid route");
    assert!(error.contains("include both plugin id and channel"));
}

#[test]
fn collect_manifest_paths_skips_symlink_loops() {
    let root = TempDir::new("manifest-loop");
    let plugins_dir = root.join("plugins");
    write_manifest(
        &plugins_dir.join("acme.search/volt-plugin.json"),
        "acme.search",
        &["fs"],
    );
    let nested = plugins_dir.join("acme.search/nested");
    fs::create_dir_all(&nested).expect("nested dir");
    create_dir_symlink(&plugins_dir.join("acme.search"), &nested.join("loop"))
        .expect("create symlink");

    let mut manifest_paths = Vec::new();
    collect_manifest_paths(&plugins_dir, &mut manifest_paths).expect("scan manifests");

    assert_eq!(manifest_paths.len(), 1);
    assert!(manifest_paths[0].ends_with("volt-plugin.json"));
}
