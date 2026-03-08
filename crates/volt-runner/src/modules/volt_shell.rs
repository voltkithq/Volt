use std::path::PathBuf;
use std::process::Command;

use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use volt_core::fs as core_fs;
use volt_core::permissions::Permission;
use volt_core::shell;

use super::{
    fs_base_dir, js_error, native_function_module, promise_from_result, require_permission,
};

fn open_external(url: String, context: &mut Context) -> JsValue {
    let result = (|| {
        require_permission(Permission::Shell).map_err(super::format_js_error)?;
        shell::open_external(&url).map_err(|error| format!("shell open failed: {error}"))
    })();

    promise_from_result(context, result).into()
}

fn show_item_in_folder(path: String) -> JsResult<()> {
    require_permission(Permission::Shell)?;
    show_item_in_folder_impl(&path)
        .map_err(|error| js_error("volt:shell", "showItemInFolder", error))
}

fn show_item_in_folder_impl(path: &str) -> Result<(), String> {
    let target_path = resolve_scoped_target_path(path)?;
    let target = target_path.as_path();

    #[cfg(target_os = "windows")]
    {
        let child = Command::new("explorer")
            .arg("/select,")
            .arg(target)
            .spawn()
            .map_err(|error| format!("failed to show item in folder: {error}"))?;
        reap_child_async(child);
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let child = Command::new("open")
            .arg("-R")
            .arg(target)
            .spawn()
            .map_err(|error| format!("failed to show item in folder: {error}"))?;
        reap_child_async(child);
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let folder = target.parent().unwrap_or(target);
        let child = Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|error| format!("failed to show item in folder: {error}"))?;
        reap_child_async(child);
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("showItemInFolder is not supported on this platform".to_string())
}

fn resolve_scoped_target_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path must not be empty".to_string());
    }

    let base_dir = fs_base_dir()?;
    let resolved = core_fs::safe_resolve(&base_dir, trimmed)
        .map_err(|error| format!("failed to resolve path within app scope: {error}"))?;
    if !resolved.exists() {
        return Err(format!(
            "path does not exist within app scope: '{}'",
            resolved.display()
        ));
    }

    Ok(resolved)
}

fn reap_child_async(mut child: std::process::Child) {
    let _ = std::thread::Builder::new()
        .name("volt-shell-child-reaper".to_string())
        .spawn(move || {
            let _ = child.wait();
        });
}

pub fn build_module(context: &mut Context) -> Module {
    let open_external = open_external.into_js_function_copied(context);
    let show_item_in_folder = show_item_in_folder.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("openExternal", open_external),
            ("showItemInFolder", show_item_in_folder),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{ModuleConfig, configure};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(prefix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "volt-shell-{prefix}-{}-{nonce}.tmp",
            std::process::id()
        ))
    }

    fn configure_shell_scope(base_dir: &std::path::Path) {
        configure(ModuleConfig {
            fs_base_dir: base_dir.to_path_buf(),
            permissions: vec!["shell".to_string()],
            ..Default::default()
        })
        .expect("configure module state");
    }

    #[test]
    fn resolve_scoped_target_path_rejects_empty_values() {
        let base_dir = std::env::temp_dir();
        configure_shell_scope(&base_dir);

        let err = resolve_scoped_target_path("   ").expect_err("empty path should fail");
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn resolve_scoped_target_path_rejects_missing_paths() {
        let base_dir =
            std::env::temp_dir().join(format!("volt-shell-scope-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base_dir);
        fs::create_dir_all(&base_dir).expect("create base dir");
        configure_shell_scope(&base_dir);

        let err = resolve_scoped_target_path("missing.txt").expect_err("missing path should fail");
        assert!(err.contains("does not exist"));

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn resolve_scoped_target_path_resolves_existing_paths_within_scope() {
        let base_dir =
            std::env::temp_dir().join(format!("volt-shell-scope-existing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base_dir);
        fs::create_dir_all(&base_dir).expect("create base dir");
        configure_shell_scope(&base_dir);

        let file_path = base_dir.join(unique_temp_path("existing").file_name().expect("file name"));
        fs::write(&file_path, "volt").expect("write temp file");

        let resolved = resolve_scoped_target_path(
            file_path
                .file_name()
                .and_then(|value| value.to_str())
                .expect("relative path"),
        )
        .expect("scoped path");
        assert_eq!(resolved, file_path.canonicalize().expect("canonical path"));

        let _ = fs::remove_dir_all(&base_dir);
    }

    #[test]
    fn resolve_scoped_target_path_rejects_absolute_paths_outside_scope() {
        let base_dir =
            std::env::temp_dir().join(format!("volt-shell-scope-absolute-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base_dir);
        fs::create_dir_all(&base_dir).expect("create base dir");
        configure_shell_scope(&base_dir);

        let outside_path = unique_temp_path("outside");
        fs::write(&outside_path, "outside").expect("write outside file");

        let err = resolve_scoped_target_path(&outside_path.to_string_lossy())
            .expect_err("absolute path should be rejected");
        assert!(err.contains("app scope"));

        let _ = fs::remove_file(outside_path);
        let _ = fs::remove_dir_all(&base_dir);
    }
}
