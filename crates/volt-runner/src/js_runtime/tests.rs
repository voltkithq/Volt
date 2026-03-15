use super::*;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IPC_HANDLER_TIMEOUT_CODE, IPC_MAX_REQUEST_BYTES};

use crate::plugin_manager::PluginManager;

mod gc_scheduler;
mod http_module;
mod native_ipc;
mod native_modules;
mod runtime_eval;

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "volt-runner-js-{prefix}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn runtime_with_permissions(fs_base_dir: PathBuf, permissions: &[&str]) -> JsRuntimeManager {
    runtime_with_plugin_manager(fs_base_dir, permissions, None)
}

pub(super) fn runtime_with_plugin_manager(
    fs_base_dir: PathBuf,
    permissions: &[&str],
    plugin_manager: Option<PluginManager>,
) -> JsRuntimeManager {
    JsRuntimeManager::start_with_options(JsRuntimeOptions {
        fs_base_dir,
        permissions: permissions.iter().map(|name| (*name).to_string()).collect(),
        app_name: "Volt Test".to_string(),
        plugin_manager,
        secure_storage_backend: None,
        updater_telemetry_enabled: false,
        updater_telemetry_sink: None,
    })
    .expect("js runtime start with permissions")
}

fn spawn_http_fixture_server(body: &'static str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    let address = listener.local_addr().expect("server addr");

    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request_buffer = [0_u8; 1024];
            let _ = stream.read(&mut request_buffer);

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{address}/"), server)
}

fn spawn_http_fixture_server_with_duplicate_headers() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    let address = listener.local_addr().expect("server addr");

    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut request_buffer = [0_u8; 1024];
            let _ = stream.read(&mut request_buffer);

            let response = "HTTP/1.1 200 OK\r\n\
Content-Type: application/json\r\n\
Set-Cookie: a=1\r\n\
Set-Cookie: b=2\r\n\
Content-Length: 11\r\n\
Connection: close\r\n\r\n\
{\"ok\":true}";
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{address}/"), server)
}

fn dispatch_ipc_request(client: &JsRuntimeClient, raw: &str) -> IpcResponse {
    client
        .dispatch_ipc_message(raw, Duration::from_secs(2))
        .expect("dispatch ipc message")
}

fn dispatch_native_event(client: &JsRuntimeClient, event_type: &str, payload: JsonValue) {
    client
        .dispatch_native_event_blocking(event_type, payload)
        .expect("dispatch native event");
}
