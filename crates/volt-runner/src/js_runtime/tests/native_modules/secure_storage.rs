use super::super::{JsRuntimeManager, JsRuntimeOptions, unique_temp_dir};
use std::fs;

#[test]
fn secure_storage_module_supports_crud_with_memory_backend() {
    let fs_base_dir = unique_temp_dir("secure-storage-crud");
    let runtime = JsRuntimeManager::start_with_options(JsRuntimeOptions {
        fs_base_dir: fs_base_dir.clone(),
        permissions: vec!["secureStorage".to_string()],
        app_name: "Volt Secure Storage Tests".to_string(),
        secure_storage_backend: Some("memory".to_string()),
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
    })
    .expect("js runtime start");

    let summary = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const secureStorage = globalThis.__volt.secureStorage;
                    await secureStorage.set('token', 's3cr3t');
                    const value = await secureStorage.get('token');
                    const hasBeforeDelete = await secureStorage.has('token');
                    await secureStorage.delete('token');
                    const hasAfterDelete = await secureStorage.has('token');
                    return `${value}:${hasBeforeDelete}:${hasAfterDelete}`;
                })()",
        )
        .expect("secure storage crud");

    assert_eq!(summary, "s3cr3t:true:false");
    let _ = fs::remove_dir_all(fs_base_dir);
}

#[test]
fn secure_storage_module_rejects_invalid_keys() {
    let fs_base_dir = unique_temp_dir("secure-storage-validation");
    let runtime = JsRuntimeManager::start_with_options(JsRuntimeOptions {
        fs_base_dir: fs_base_dir.clone(),
        permissions: vec!["secureStorage".to_string()],
        app_name: "Volt Secure Storage Tests".to_string(),
        secure_storage_backend: Some("memory".to_string()),
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
    })
    .expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const secureStorage = globalThis.__volt.secureStorage;
                    const errors = [];

                    for (const operation of [
                        () => secureStorage.set('', 'x'),
                        () => secureStorage.get(''),
                        () => secureStorage.delete(''),
                        () => secureStorage.has(''),
                    ]) {
                        try {
                            await operation();
                            errors.push(false);
                        } catch (error) {
                            errors.push(String(error).includes('must not be empty'));
                        }
                    }

                    return errors.every(Boolean) ? 'invalid' : 'unexpected';
                })()",
        )
        .expect("secure storage validation");

    assert_eq!(outcome, "invalid");
    let _ = fs::remove_dir_all(fs_base_dir);
}

#[test]
fn secure_storage_module_rejects_without_permission() {
    let fs_base_dir = unique_temp_dir("secure-storage-permission");
    let runtime = JsRuntimeManager::start_with_options(JsRuntimeOptions {
        fs_base_dir: fs_base_dir.clone(),
        permissions: Vec::new(),
        app_name: "Volt Secure Storage Tests".to_string(),
        secure_storage_backend: Some("memory".to_string()),
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
    })
    .expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const secureStorage = globalThis.__volt.secureStorage;
                    try {
                        await secureStorage.set('token', 'value');
                        return 'unexpected';
                    } catch (error) {
                        return String(error).includes('Permission denied') ? 'denied' : String(error);
                    }
                })()",
        )
        .expect("secure storage permission");

    assert_eq!(outcome, "denied");
    let _ = fs::remove_dir_all(fs_base_dir);
}
