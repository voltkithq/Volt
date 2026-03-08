use super::super::{runtime_with_permissions, unique_temp_dir};
use std::fs;

#[test]
fn integration_script_uses_multiple_modules_together() {
    let fs_base_dir = unique_temp_dir("integration-modules");
    let runtime = runtime_with_permissions(fs_base_dir.clone(), &["fs"]);

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const fs = globalThis.__volt.fs;
                    const crypto = globalThis.__volt.crypto;
                    const os = globalThis.__volt.os;

                    const hash = crypto.sha256(os.platform());
                    await fs.writeFile('combo.txt', hash);
                    const loaded = await fs.readFile('combo.txt');
                    await fs.remove('combo.txt');
                    return loaded.length > 0 ? 'ok' : 'bad';
                })()",
        )
        .expect("integration module script");

    assert_eq!(outcome, "ok");
    let _ = fs::remove_dir_all(fs_base_dir);
}
