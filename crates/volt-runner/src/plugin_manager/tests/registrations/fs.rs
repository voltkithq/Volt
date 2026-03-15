use std::sync::{Arc, Mutex};

use serde_json::json;

use super::super::super::PLUGIN_FS_ERROR_CODE;
use super::*;

#[test]
fn plugin_fs_requests_reject_path_traversal() {
    let (manager, _factory) = manager_for_registration_tests(
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );

    let response = manager
        .handle_plugin_message(
            "acme.search",
            request_message(
                "req-1",
                "plugin:fs:read-file",
                json!({ "path": "../escape.txt" }),
            ),
        )
        .expect("fs response");

    let error = response.error.expect("fs traversal should fail");
    assert_eq!(error.code, PLUGIN_FS_ERROR_CODE);
}
