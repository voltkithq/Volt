#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use serde_json::json;
use volt_core::app::{App, AppConfig, AppEvent};

mod ipc_bridge;
mod js_runtime;
mod js_runtime_pool;
mod modules;
mod runner;

use runner::RunnerError;
use runner::assets::{load_asset_bundle, load_backend_bundle_source};
use runner::config::load_runner_config;
use runner::fs::resolve_fs_scope_dir;

mod logging;

fn main() {
    logging::init_logging(if cfg!(debug_assertions) {
        "debug"
    } else {
        "warn"
    });
    if let Err(err) = run() {
        tracing::error!(error = %err, "volt-runner exited with error");
        std::process::exit(1);
    }
}

fn run() -> Result<(), RunnerError> {
    let config = load_runner_config()?;
    let startup_recovery = modules::volt_updater::prepare_startup_recovery()
        .map_err(|error| RunnerError::App(format!("updater startup recovery failed: {error}")))?;
    let bundle = load_asset_bundle()?;
    let backend_bundle_source = load_backend_bundle_source()?;
    let pool_size = config
        .runtime_pool_size
        .unwrap_or_else(js_runtime_pool::JsRuntimePool::default_pool_size);
    let js_runtime = js_runtime_pool::JsRuntimePool::start_with_options(
        pool_size,
        js_runtime::JsRuntimeOptions {
            fs_base_dir: resolve_fs_scope_dir(&config)?,
            permissions: config.permissions.clone(),
            app_name: config.app_name.clone(),
            secure_storage_backend: None,
            updater_telemetry_enabled: config.updater_telemetry_enabled,
            updater_telemetry_sink: config.updater_telemetry_sink.clone(),
        },
    )
    .map_err(|err| RunnerError::App(format!("failed to start JS runtime pool: {err}")))?;
    js_runtime
        .client()
        .eval_i64("40 + 2")
        .map_err(|err| RunnerError::App(format!("JS runtime sanity check failed: {err}")))?;
    js_runtime
        .client()
        .load_backend_bundle(&backend_bundle_source)
        .map_err(|err| RunnerError::App(format!("failed to load backend bundle: {err}")))?;
    let runtime_client = js_runtime.client();
    let ipc_bridge = ipc_bridge::IpcBridge::new(runtime_client.clone());

    let mut app = App::new(AppConfig {
        name: config.app_name,
        devtools: config.devtools,
    })
    .map_err(|err| RunnerError::App(format!("failed to create app: {err}")))?;

    app.set_asset_bundle(bundle);
    app.create_window(config.window, config.webview)
        .map_err(|err| RunnerError::App(format!("failed to create window: {err}")))?;
    if let Some(healthy_startup_window) = startup_recovery {
        modules::volt_updater::spawn_healthy_startup_clearer(healthy_startup_window);
    }
    let app_result = app.run(move |event| match event {
        AppEvent::IpcMessage { js_window_id, raw } => {
            ipc_bridge.handle_message(js_window_id.clone(), raw.clone());
        }
        AppEvent::MenuEvent { menu_id } => {
            if let Err(error) =
                runtime_client.dispatch_native_event("menu:click", json!({ "menuId": menu_id }))
            {
                tracing::error!(error = %error, menu_id = %menu_id, "failed to dispatch menu event");
            }
        }
        AppEvent::ShortcutTriggered { id } => {
            if let Err(error) =
                runtime_client.dispatch_native_event("shortcut:triggered", json!({ "id": id }))
            {
                tracing::error!(error = %error, shortcut_id = id, "failed to dispatch shortcut event");
            }
        }
        AppEvent::TrayEvent { tray_id } => {
            if let Err(error) =
                runtime_client.dispatch_native_event("tray:click", json!({ "trayId": tray_id }))
            {
                tracing::error!(error = %error, tray_id = %tray_id, "failed to dispatch tray event");
            }
        }
        _ => {}
    });

    // Keep the JS runtime worker alive for the full native event loop lifetime.
    drop(js_runtime);
    app_result.map_err(|err| RunnerError::App(format!("event loop error: {err}")))?;

    Ok(())
}
