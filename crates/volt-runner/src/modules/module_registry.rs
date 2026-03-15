use boa_engine::module::{MapModuleLoader, Module};
use boa_engine::{Context, JsResult};

use super::{
    volt_bench, volt_clipboard, volt_crypto, volt_db, volt_dialog, volt_events, volt_fs,
    volt_global_shortcut, volt_http, volt_ipc, volt_menu, volt_notification, volt_os, volt_plugins,
    volt_secure_storage, volt_shell, volt_tray, volt_updater, volt_window,
};

pub struct RegisteredModule {
    pub global_name: &'static str,
    pub specifier: &'static str,
    pub module: Module,
}

fn register_module(
    module_loader: &MapModuleLoader,
    registered_modules: &mut Vec<RegisteredModule>,
    global_name: &'static str,
    specifier: &'static str,
    module: Module,
) {
    module_loader.insert(specifier, module.clone());
    registered_modules.push(RegisteredModule {
        global_name,
        specifier,
        module,
    });
}

pub fn register_all_modules(
    context: &mut Context,
    module_loader: &MapModuleLoader,
) -> JsResult<Vec<RegisteredModule>> {
    let mut registered_modules = Vec::with_capacity(19);

    register_module(
        module_loader,
        &mut registered_modules,
        "fs",
        "volt:fs",
        volt_fs::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "db",
        "volt:db",
        volt_db::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "bench",
        "volt:bench",
        volt_bench::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "dialog",
        "volt:dialog",
        volt_dialog::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "events",
        "volt:events",
        volt_events::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "clipboard",
        "volt:clipboard",
        volt_clipboard::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "shell",
        "volt:shell",
        volt_shell::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "notification",
        "volt:notification",
        volt_notification::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "menu",
        "volt:menu",
        volt_menu::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "http",
        "volt:http",
        volt_http::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "globalShortcut",
        "volt:globalShortcut",
        volt_global_shortcut::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "ipc",
        "volt:ipc",
        volt_ipc::build_module(context)?,
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "secureStorage",
        "volt:secureStorage",
        volt_secure_storage::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "tray",
        "volt:tray",
        volt_tray::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "crypto",
        "volt:crypto",
        volt_crypto::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "updater",
        "volt:updater",
        volt_updater::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "window",
        "volt:window",
        volt_window::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "os",
        "volt:os",
        volt_os::build_module(context),
    );
    register_module(
        module_loader,
        &mut registered_modules,
        "plugins",
        "volt:plugins",
        volt_plugins::build_module(context),
    );

    Ok(registered_modules)
}
