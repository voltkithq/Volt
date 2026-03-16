use std::path::Path;
use std::rc::Rc;

use boa_engine::Context;
use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::module::{MapModuleLoader, Module};
use boa_engine::object::builtins::JsPromise;
use boa_engine::{JsValue, Source, js_string};
use tokio::runtime::Builder as TokioRuntimeBuilder;
use tokio::task::yield_now;

use crate::config::PluginConfig;
use crate::ipc::{IpcMessage, MessageType};
use crate::{modules, runtime_state};

pub struct PluginEngine {
    context: Context,
    job_executor: Rc<SimpleJobExecutor>,
    tokio_runtime: tokio::runtime::Runtime,
}

impl PluginEngine {
    pub fn start(config: &PluginConfig) -> Result<Self, String> {
        Self::start_inner(config, true)
    }

    #[cfg(test)]
    pub fn start_with_mock(config: &PluginConfig) -> Result<Self, String> {
        Self::start_inner(config, false)
    }

    fn start_inner(config: &PluginConfig, configure_stdio: bool) -> Result<Self, String> {
        if configure_stdio {
            runtime_state::configure_stdio(config);
        }

        let tokio_runtime = TokioRuntimeBuilder::new_current_thread()
            .enable_time()
            .build()
            .map_err(|error| format!("failed to create tokio runtime: {error}"))?;
        let job_executor = Rc::new(SimpleJobExecutor::new());
        let module_loader = Rc::new(MapModuleLoader::default());
        let mut context = Context::builder()
            .module_loader(module_loader.clone())
            .job_executor(job_executor.clone())
            .build()
            .map_err(|error| format!("failed to create Boa context: {error}"))?;

        modules::volt_plugin::register_native_bridge(&mut context)
            .map_err(|error| error.to_string())?;
        let module =
            modules::volt_plugin::build_module(&mut context).map_err(|error| error.to_string())?;
        module_loader.insert("volt:plugin", module);

        let mut engine = Self {
            context,
            job_executor,
            tokio_runtime,
        };
        engine.load_backend(config)?;
        runtime_state::send_signal("init", "ready")?;
        Ok(engine)
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let Some(message) = runtime_state::next_message().map_err(|error| error.to_string())?
            else {
                tracing::info!("stdin EOF - exiting plugin event loop");
                return Ok(());
            };

            if self.dispatch_message(message)? {
                return Ok(());
            }
        }
    }

    fn load_backend(&mut self, config: &PluginConfig) -> Result<(), String> {
        let script = std::fs::read_to_string(&config.backend_entry).map_err(|error| {
            format!(
                "failed to read plugin backend '{}': {error}",
                config.backend_entry
            )
        })?;
        if script.trim().is_empty() {
            return Ok(());
        }

        let source =
            Source::from_bytes(script.as_str()).with_path(Path::new(&config.backend_entry));
        let module =
            Module::parse(source, None, &mut self.context).map_err(|error| error.to_string())?;
        let promise = module.load_link_evaluate(&mut self.context);
        let _ = self.settle_promise(promise)?;
        Ok(())
    }

    pub fn dispatch_message(&mut self, message: IpcMessage) -> Result<bool, String> {
        match (message.msg_type, message.method.as_str()) {
            (MessageType::Signal, "heartbeat") => {
                runtime_state::send_signal(message.id, "heartbeat-ack")?;
                Ok(false)
            }
            (MessageType::Signal, "activate") => {
                self.respond_to_async_call(
                    message.id,
                    "activate",
                    "__volt_plugin_activate__",
                    &[],
                )?;
                Ok(false)
            }
            (MessageType::Signal, "deactivate") => {
                self.respond_to_async_call(
                    message.id,
                    "deactivate",
                    "__volt_plugin_deactivate__",
                    &[],
                )?;
                Ok(true)
            }
            (MessageType::Request, "plugin:invoke-command") => {
                let payload = message.payload.unwrap_or(serde_json::Value::Null);
                let args = [
                    JsValue::from(js_string!(required_string(&payload, "id")?.as_str())),
                    json_arg(&payload, "args", &mut self.context)?,
                ];
                self.respond_to_async_call(
                    message.id,
                    "plugin:invoke-command",
                    "__volt_plugin_invoke_command__",
                    &args,
                )?;
                Ok(false)
            }
            (MessageType::Request, "plugin:invoke-ipc") => {
                let payload = message.payload.unwrap_or(serde_json::Value::Null);
                let args = [
                    JsValue::from(js_string!(required_string(&payload, "channel")?.as_str())),
                    json_arg(&payload, "args", &mut self.context)?,
                ];
                self.respond_to_async_call(
                    message.id,
                    "plugin:invoke-ipc",
                    "__volt_plugin_invoke_ipc__",
                    &args,
                )?;
                Ok(false)
            }
            (MessageType::Event, "plugin:event") => {
                let payload = message.payload.unwrap_or(serde_json::Value::Null);
                let args = [
                    JsValue::from(js_string!(required_string(&payload, "event")?.as_str())),
                    json_arg(&payload, "data", &mut self.context)?,
                ];
                let _ = self.call_async_global("__volt_plugin_dispatch_event__", &args)?;
                Ok(false)
            }
            (MessageType::Event, "plugin:grant-revoked") => {
                let payload = message.payload.unwrap_or(serde_json::Value::Null);
                let args = [JsValue::from(js_string!(
                    required_string(&payload, "grantId")?.as_str()
                ))];
                let _ = self.call_async_global("__volt_plugin_revoke_grant__", &args)?;
                Ok(false)
            }
            (MessageType::Response, _)
            | (MessageType::Event, _)
            | (MessageType::Signal, "cancel") => Ok(false),
            (MessageType::Signal, _) | (MessageType::Request, _) => {
                runtime_state::send_error(
                    message.id,
                    message.method,
                    "UNHANDLED",
                    "message is not supported by the plugin runtime",
                )?;
                Ok(false)
            }
        }
    }

    fn respond_to_async_call(
        &mut self,
        id: String,
        method: &str,
        global_name: &str,
        args: &[JsValue],
    ) -> Result<(), String> {
        match self.call_async_global(global_name, args) {
            Ok(value) => runtime_state::send_response(
                id,
                method,
                Some(js_to_json(&mut self.context, value)?),
            ),
            Err(error) => runtime_state::send_error(id, method, "PLUGIN_RUNTIME_ERROR", error),
        }
    }

    fn call_async_global(
        &mut self,
        global_name: &str,
        args: &[JsValue],
    ) -> Result<JsValue, String> {
        let callable = self
            .context
            .global_object()
            .get(js_string!(global_name), &mut self.context)
            .map_err(|error| error.to_string())?;
        let callable = callable
            .as_callable()
            .ok_or_else(|| format!("global '{global_name}' is not callable"))?;
        let result = callable
            .call(&JsValue::undefined(), args, &mut self.context)
            .map_err(|error| error.to_string())?;

        if let Some(object) = result.as_object()
            && let Ok(promise) = JsPromise::from_object(object)
        {
            return self.settle_promise(promise);
        }

        Ok(result)
    }

    fn settle_promise(&mut self, promise: JsPromise) -> Result<JsValue, String> {
        self.tokio_runtime.block_on(async {
            loop {
                let context_cell = std::cell::RefCell::new(&mut self.context);
                boa_engine::job::JobExecutor::run_jobs_async(
                    self.job_executor.clone(),
                    &context_cell,
                )
                .await
                .map_err(|error| error.to_string())?;

                match promise.state() {
                    PromiseState::Pending => {
                        yield_now().await;
                        continue;
                    }
                    PromiseState::Fulfilled(value) => return Ok(value),
                    PromiseState::Rejected(error) => return Err(error.display().to_string()),
                }
            }
        })
    }
}

fn required_string(payload: &serde_json::Value, key: &str) -> Result<String, String> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("payload is missing required '{key}' string"))
}

fn json_arg(
    payload: &serde_json::Value,
    key: &str,
    context: &mut Context,
) -> Result<JsValue, String> {
    let value = payload.get(key).cloned().unwrap_or(serde_json::Value::Null);
    JsValue::from_json(&value, context).map_err(|error| error.to_string())
}

fn js_to_json(context: &mut Context, value: JsValue) -> Result<serde_json::Value, String> {
    value
        .to_json(context)
        .map(|value| value.unwrap_or(serde_json::Value::Null))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
