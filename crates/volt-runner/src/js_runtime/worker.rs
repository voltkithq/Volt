use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use boa_engine::Context;
use boa_engine::gc;
use boa_engine::job::SimpleJobExecutor;
use boa_engine::module::MapModuleLoader;
use tokio::runtime::Builder as TokioRuntimeBuilder;

use super::{JsRuntimeOptions, RuntimeRequest};

pub(super) struct GcScheduler {
    pub(super) last_gc_at: Instant,
    pub(super) requests_since_gc: usize,
}

impl GcScheduler {
    pub(super) fn new() -> Self {
        Self {
            last_gc_at: Instant::now(),
            requests_since_gc: 0,
        }
    }

    pub(super) fn note_request_completed(&mut self) {
        self.requests_since_gc = self.requests_since_gc.saturating_add(1);
    }

    pub(super) fn should_run_gc(&self) -> bool {
        self.requests_since_gc >= super::JS_GC_REQUEST_INTERVAL
            || self.last_gc_at.elapsed() >= super::JS_GC_INTERVAL
    }

    pub(super) fn mark_gc_run(&mut self) {
        self.requests_since_gc = 0;
        self.last_gc_at = Instant::now();
    }
}

pub(super) fn worker_main(
    request_rx: Receiver<RuntimeRequest>,
    ready_tx: Sender<Result<(), String>>,
    options: JsRuntimeOptions,
) {
    let tokio_runtime = match TokioRuntimeBuilder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            let _ = ready_tx.send(Err(format!("failed to create tokio runtime: {err}")));
            return;
        }
    };

    let module_loader = Rc::new(MapModuleLoader::default());
    let job_executor = Rc::new(SimpleJobExecutor::new());
    // Boa guarantees module_loader and job_executor are active if build() succeeds.
    // If a future Boa version changes this contract, context creation will fail here.
    let mut context = match Context::builder()
        .module_loader(module_loader.clone())
        .job_executor(job_executor.clone())
        .build()
    {
        Ok(context) => context,
        Err(err) => {
            let _ = ready_tx.send(Err(format!("failed to create Boa context: {err}")));
            return;
        }
    };

    let init_result = tokio_runtime.block_on(async {
        super::bootstrap::initialize_context(&mut context, &module_loader, &job_executor, options)
            .await?;
        run_startup_sanity_check(&mut context, &job_executor).await
    });

    if let Err(err) = init_result {
        let _ = ready_tx.send(Err(err));
        return;
    }

    if ready_tx.send(Ok(())).is_err() {
        return;
    }

    let mut ipc_state = super::ipc::IpcRuntimeState::default();
    let mut gc_scheduler = GcScheduler::new();
    for request in request_rx {
        if matches!(request, RuntimeRequest::Shutdown) {
            break;
        }
        execute_request(
            &tokio_runtime,
            &job_executor,
            &mut context,
            &mut ipc_state,
            &mut gc_scheduler,
            request,
        );
    }
}

async fn run_startup_sanity_check(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
) -> Result<(), String> {
    let value = super::eval_i64(context, "21 + 21").await?;
    if value != 42 {
        return Err(format!(
            "unexpected js runtime startup sanity value: expected 42, got {value}"
        ));
    }
    super::bootstrap::run_jobs(context, job_executor).await
}

fn execute_request(
    tokio_runtime: &tokio::runtime::Runtime,
    job_executor: &Rc<SimpleJobExecutor>,
    context: &mut Context,
    ipc_state: &mut super::ipc::IpcRuntimeState,
    gc_scheduler: &mut GcScheduler,
    request: RuntimeRequest,
) {
    match request {
        RuntimeRequest::EvalI64 {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_i64(context, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        #[cfg(test)]
        RuntimeRequest::EvalBool {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_bool(context, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        #[cfg(test)]
        RuntimeRequest::EvalString {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_string(context, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        #[cfg(test)]
        RuntimeRequest::EvalPromiseI64 {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_promise_i64(context, job_executor, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        #[cfg(test)]
        RuntimeRequest::EvalPromiseString {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_promise_string(context, job_executor, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        #[cfg(test)]
        RuntimeRequest::EvalUnit {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_unit(context, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        RuntimeRequest::LoadBackendBundle {
            script,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::eval_backend_bundle(context, job_executor, &script).await;
                finalize_request(context, job_executor, result).await
            });
            let _ = response_tx.send(result);
        }
        RuntimeRequest::DispatchIpc {
            raw,
            timeout,
            response_tx,
        } => {
            let response = tokio_runtime.block_on(async {
                let response = super::ipc::dispatch_ipc_request(
                    context,
                    job_executor,
                    ipc_state,
                    &raw,
                    timeout,
                )
                .await;
                let _ = super::bootstrap::run_jobs(context, job_executor).await;
                response
            });
            let _ = response_tx.send(response);
        }
        RuntimeRequest::DispatchNativeEvent {
            event_type,
            payload,
            response_tx,
        } => {
            let result = tokio_runtime.block_on(async {
                let result = super::native_events::dispatch_native_event_handler(
                    context,
                    job_executor,
                    &event_type,
                    &payload,
                )
                .await;
                finalize_request(context, job_executor, result).await
            });

            if let Some(response_tx) = response_tx {
                let _ = response_tx.send(result);
            } else if let Err(error) = result {
                tracing::error!(
                    error = %error,
                    event_type = %event_type,
                    "failed to dispatch native event"
                );
            }
        }
        RuntimeRequest::Shutdown => {}
    }

    gc_scheduler.note_request_completed();
    maybe_run_gc(tokio_runtime, job_executor, context, gc_scheduler);
}

async fn finalize_request<T>(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
    result: Result<T, String>,
) -> Result<T, String> {
    let idle_result = super::bootstrap::run_jobs(context, job_executor).await;
    match (result, idle_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

fn maybe_run_gc(
    tokio_runtime: &tokio::runtime::Runtime,
    job_executor: &Rc<SimpleJobExecutor>,
    context: &mut Context,
    gc_scheduler: &mut GcScheduler,
) {
    if !gc_scheduler.should_run_gc() {
        return;
    }

    let gc_start = Instant::now();
    gc::force_collect();
    if let Err(error) = tokio_runtime.block_on(super::bootstrap::run_jobs(context, job_executor)) {
        tracing::warn!(error = %error, "failed to run jobs during garbage collection");
    }
    tracing::debug!(
        elapsed_ms = gc_start.elapsed().as_millis(),
        "garbage collection completed"
    );
    gc_scheduler.mark_gc_run();
}
