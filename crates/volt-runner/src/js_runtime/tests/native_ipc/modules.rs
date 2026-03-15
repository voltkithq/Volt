use super::*;

#[test]
fn ipc_handler_can_use_native_modules() {
    let fs_base_dir = unique_temp_dir("ipc-fs-handler");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["fs"]);
    let client = runtime.client();

    client
        .eval_promise_string(
            "(async () => {
                    const { ipcMain } = await import('volt:ipc');
                    const fs = globalThis.__volt.fs;
                    ipcMain.handle('save-document', async (args) => {
                        await fs.writeFile(args.path, args.content);
                        const loaded = await fs.readFile(args.path);
                        await fs.remove(args.path);
                        return { saved: true, loaded };
                    });
                    return 'registered';
                })()",
        )
        .expect("register fs ipc handler");

    let response = dispatch_ipc_request(
        &client,
        r#"{"id":"fs-1","method":"save-document","args":{"path":"doc.txt","content":"hello"}}"#,
    );
    assert_eq!(response.id, "fs-1");
    assert_eq!(
        response.result,
        Some(serde_json::json!({ "saved": true, "loaded": "hello" }))
    );
    assert!(response.error.is_none());

    let _ = std::fs::remove_dir_all(fs_base_dir);
}
