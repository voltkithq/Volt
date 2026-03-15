use std::rc::Rc;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::object::JsObject;
use boa_engine::property::Attribute;
use boa_engine::{Context, js_string};

use crate::modules::RegisteredModule;

pub(crate) async fn run_jobs(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
) -> Result<(), String> {
    let context_cell = std::cell::RefCell::new(context);
    boa_engine::job::JobExecutor::run_jobs_async(job_executor.clone(), &context_cell)
        .await
        .map_err(super::super::serde_support::js_error)
}

async fn load_module_namespace(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    registered_module: &RegisteredModule,
) -> Result<JsObject, String> {
    let promise = registered_module.module.load_link_evaluate(context);
    run_jobs(context, job_executor).await?;

    match promise.state() {
        PromiseState::Pending => Err("evaluation did not settle".to_string()),
        PromiseState::Fulfilled(_) => Ok(registered_module.module.namespace(context)),
        PromiseState::Rejected(error) => Err(super::super::serde_support::js_value_to_string(
            context, error,
        )?),
    }
}

pub(super) async fn expose_native_modules_on_global(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    registered_modules: &[RegisteredModule],
) -> Result<(), String> {
    let volt_modules = JsObject::with_null_proto();
    for registered_module in registered_modules {
        let namespace = load_module_namespace(context, job_executor, registered_module)
            .await
            .map_err(|error| {
                format!(
                    "failed to load module '{}': {error}",
                    registered_module.specifier
                )
            })?;
        volt_modules
            .set(
                js_string!(registered_module.global_name),
                namespace,
                true,
                context,
            )
            .map_err(super::super::serde_support::js_error)?;
    }

    context
        .register_global_property(js_string!("__volt"), volt_modules, Attribute::all())
        .map_err(super::super::serde_support::js_error)
}
