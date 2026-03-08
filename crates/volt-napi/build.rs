fn main() {
    // On windows-gnu, napi-build's setup() panics looking for libnode.dll.
    // However, napi v3 uses dynamic symbol loading (GetProcAddress) on GNU targets,
    // so link-time resolution against libnode.dll is not required for .node addons.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_os == "windows" && target_env == "gnu" {
        return;
    }

    napi_build::setup();
}
