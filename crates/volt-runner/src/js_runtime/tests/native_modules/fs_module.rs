use super::super::{JsRuntimeManager, JsRuntimeOptions, runtime_with_permissions, unique_temp_dir};
use std::fs;

#[test]
fn fs_module_operations_work_with_permission() {
    let fs_base_dir = unique_temp_dir("fs-module");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["fs"]);

    let summary = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const fs = globalThis.__volt.fs;
                    await fs.mkdir('sandbox');
                    await fs.writeFile('sandbox/note.txt', 'hello');
                    const exists = await fs.exists('sandbox/note.txt');
                    const content = await fs.readFile('sandbox/note.txt');
                    const names = await fs.readDir('sandbox');
                    await fs.remove('sandbox/note.txt');
                    await fs.remove('sandbox');
                    return `${exists}:${content}:${names.includes('note.txt')}`;
                })()",
        )
        .expect("fs module usage");
    assert_eq!(summary, "true:hello:true");

    let traversal = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const fs = globalThis.__volt.fs;
                    try {
                        await fs.readFile('../outside.txt');
                        return 'unexpected';
                    } catch (_) {
                        return 'blocked';
                    }
                })()",
        )
        .expect("fs traversal validation");
    assert_eq!(traversal, "blocked");

    let _ = fs::remove_dir_all(fs_base_dir);
}

#[test]
fn fs_module_rejects_without_permission() {
    let fs_base_dir = unique_temp_dir("fs-perm-denied");
    let runtime = JsRuntimeManager::start_with_options(JsRuntimeOptions {
        fs_base_dir: fs_base_dir.clone(),
        permissions: Vec::new(),
        app_name: "Volt Test".to_string(),
        secure_storage_backend: None,
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
    })
    .expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const fs = globalThis.__volt.fs;
                    try {
                        await fs.readFile('note.txt');
                        return 'unexpected';
                    } catch (error) {
                        return String(error).includes('Permission denied') ? 'denied' : String(error);
                    }
                })()",
        )
        .expect("permission script");
    assert_eq!(outcome, "denied");

    let _ = fs::remove_dir_all(fs_base_dir);
}
