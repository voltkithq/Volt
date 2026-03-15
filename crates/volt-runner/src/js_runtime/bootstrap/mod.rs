use std::rc::Rc;

use boa_engine::Context;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::module::MapModuleLoader;

use crate::modules as runtime_modules;

use super::JsRuntimeOptions;

mod console;
mod modules;
mod native_events;
mod timers;

pub(crate) use self::modules::run_jobs;

pub(super) async fn initialize_context(
    context: &mut Context,
    module_loader: &MapModuleLoader,
    job_executor: &Rc<SimpleJobExecutor>,
    options: JsRuntimeOptions,
) -> Result<(), String> {
    runtime_modules::configure(runtime_modules::ModuleConfig {
        fs_base_dir: options.fs_base_dir,
        permissions: options.permissions,
        app_name: options.app_name,
        plugin_manager: options.plugin_manager,
        secure_storage_backend: options.secure_storage_backend,
        updater_telemetry_enabled: options.updater_telemetry_enabled,
        updater_telemetry_sink: options.updater_telemetry_sink,
    })?;

    console::register_console(context).map_err(super::serde_support::js_error)?;
    timers::register_timers(context).map_err(super::serde_support::js_error)?;
    let registered_modules = runtime_modules::register_all_modules(context, module_loader)
        .map_err(super::serde_support::js_error)?;
    modules::expose_native_modules_on_global(context, job_executor, &registered_modules).await?;
    native_events::register_native_event_bridge_globals(context)
        .map_err(super::serde_support::js_error)?;
    #[cfg(test)]
    timers::register_native_async_helpers(context).map_err(super::serde_support::js_error)?;
    Ok(())
}
