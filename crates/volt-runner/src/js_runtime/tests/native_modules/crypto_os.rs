use super::super::JsRuntimeManager;

#[test]
fn crypto_and_os_modules_can_be_imported() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");
    let client = runtime.client();

    let digest = client
        .eval_promise_string(
            "(async () => { const crypto = globalThis.__volt.crypto; return crypto.sha256('abc'); })()",
        )
        .expect("crypto import");
    assert_eq!(
        digest,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let roundtrip = client
        .eval_promise_string(
            "(async () => {
                    const crypto = globalThis.__volt.crypto;
                    const encoded = crypto.base64Encode('volt');
                    return crypto.base64Decode(encoded);
                })()",
        )
        .expect("base64 roundtrip");
    assert_eq!(roundtrip, "volt");

    let invalid_decode = client
        .eval_promise_string(
            "(async () => {
                    const crypto = globalThis.__volt.crypto;
                    try {
                        crypto.base64Decode('***');
                        return 'unexpected';
                    } catch (_) {
                        return 'invalid';
                    }
                })()",
        )
        .expect("base64 invalid decode");
    assert_eq!(invalid_decode, "invalid");

    let platform_arch = client
        .eval_promise_string(
            "(async () => { const os = globalThis.__volt.os; return `${os.platform()}:${os.arch()}`; })()",
        )
        .expect("os import");
    assert!(platform_arch.contains(':'));

    let dir_types = client
        .eval_promise_string(
            "(async () => {
                    const os = globalThis.__volt.os;
                    return `${typeof os.homeDir()}:${typeof os.tempDir()}`;
                })()",
        )
        .expect("os directory helpers");
    assert_eq!(dir_types, "string:string");

    let updater_shape = client
        .eval_promise_string(
            "(async () => {
                    const updater = globalThis.__volt.updater;
                    return `${typeof updater.checkForUpdate}:${typeof updater.downloadAndInstall}`;
                })()",
        )
        .expect("updater import");
    assert_eq!(updater_shape, "function:function");

    let secure_storage_shape = client
        .eval_promise_string(
            "(async () => {
                    const secureStorage = globalThis.__volt.secureStorage;
                    return `${typeof secureStorage.set}:${typeof secureStorage.get}:${typeof secureStorage.delete}:${typeof secureStorage.has}`;
                })()",
        )
        .expect("secure storage import");
    assert_eq!(secure_storage_shape, "function:function:function:function");
}
