use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::now_ms;

static UNIQUE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_unique_id() -> u64 {
    UNIQUE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub(super) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(super) fn new(name: &str) -> Self {
        let sequence = next_unique_id();
        let path = std::env::temp_dir().join(format!(
            "volt-plugin-manager-{name}-{}-{sequence}",
            now_ms()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    pub(super) fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(unix)]
pub(super) fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

#[cfg(windows)]
pub(super) fn create_dir_symlink(original: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(original, link)
}

pub(super) fn unique_app_name(prefix: &str) -> String {
    format!("{prefix} {}-{}", now_ms(), next_unique_id())
}

pub(super) fn write_manifest(path: &Path, id: &str, capabilities: &[&str]) {
    let manifest = serde_json::json!({
        "id": id,
        "name": "Test Plugin",
        "version": "0.1.0",
        "apiVersion": 1,
        "engine": { "volt": "^0.1.0" },
        "backend": "./dist/plugin.js",
        "capabilities": capabilities
    });
    fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
    fs::write(path, serde_json::to_vec(&manifest).expect("manifest json")).expect("manifest");
    let backend = path
        .parent()
        .expect("manifest parent")
        .join("dist")
        .join("plugin.js");
    fs::create_dir_all(backend.parent().expect("backend parent")).expect("backend dir");
    fs::write(backend, b"export default {};\n").expect("backend");
}
