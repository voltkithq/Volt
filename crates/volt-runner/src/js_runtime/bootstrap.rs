use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use boa_engine::builtins::promise::PromiseState;
use boa_engine::job::{JobExecutor, SimpleJobExecutor};
use boa_engine::module::MapModuleLoader;
use boa_engine::native_function::NativeFunction;
use boa_engine::object::{JsObject, ObjectInitializer};
use boa_engine::property::Attribute;
use boa_engine::{Context, JsArgs, JsResult, JsValue, Source, js_string};

use crate::modules::{self, RegisteredModule};

use super::JsRuntimeOptions;

const TIMER_BOOTSTRAP: &str = r#"
(() => {
    const timers = new Map();
    let nextTimerId = 1;

    const normalizeDelay = (value) => {
        const numeric = Number(value);
        if (!Number.isFinite(numeric) || numeric < 0) {
            return 0;
        }
        return Math.trunc(numeric);
    };

    const ensureCallback = (callback, name) => {
        if (typeof callback !== 'function') {
            throw new TypeError(`${name} callback must be a function`);
        }
        return callback;
    };

    const clearTimer = (handle) => {
        timers.delete(Number(handle));
    };

    const schedule = (callback, delay, args, repeat) => {
        const timerId = nextTimerId++;
        timers.set(timerId, {
            callback: ensureCallback(callback, repeat ? 'setInterval' : 'setTimeout'),
            delay: normalizeDelay(delay),
            repeat,
            args,
        });

        const runner = async () => {
            while (timers.has(timerId)) {
                const current = timers.get(timerId);
                if (!current) {
                    return;
                }

                await __volt_native_sleep__(current.delay);

                const active = timers.get(timerId);
                if (!active) {
                    return;
                }

                try {
                    active.callback(...active.args);
                } catch (error) {
                    console.error('[volt] timer callback failed', error);
                }

                if (!active.repeat) {
                    timers.delete(timerId);
                    return;
                }
            }
        };

        void runner();
        return timerId;
    };

    Object.defineProperty(globalThis, 'setTimeout', {
        configurable: true,
        enumerable: false,
        writable: true,
        value: (callback, delay = 0, ...args) => schedule(callback, delay, args, false),
    });

    Object.defineProperty(globalThis, 'clearTimeout', {
        configurable: true,
        enumerable: false,
        writable: true,
        value: clearTimer,
    });

    Object.defineProperty(globalThis, 'setInterval', {
        configurable: true,
        enumerable: false,
        writable: true,
        value: (callback, delay = 0, ...args) => schedule(callback, delay, args, true),
    });

    Object.defineProperty(globalThis, 'clearInterval', {
        configurable: true,
        enumerable: false,
        writable: true,
        value: clearTimer,
    });
})();
"#;

const NATIVE_EVENT_BOOTSTRAP: &str = r#"
(() => {
    const handlers = new Map();

    const normalizeEventType = (eventType) => {
        if (typeof eventType !== 'string') {
            throw new TypeError('Native event type must be a string');
        }
        const normalized = eventType.trim();
        if (!normalized) {
            throw new TypeError('Native event type must not be empty');
        }
        return normalized;
    };

    const ensureHandler = (handler) => {
        if (typeof handler !== 'function') {
            throw new TypeError('Native event handler must be a function');
        }
        return handler;
    };

    const getHandlers = (eventType) => {
        const key = normalizeEventType(eventType);
        let listeners = handlers.get(key);
        if (!listeners) {
            listeners = new Set();
            handlers.set(key, listeners);
        }
        return { key, listeners };
    };

    Object.defineProperty(globalThis, '__volt_native_event_on__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: (eventType, handler) => {
            const listener = ensureHandler(handler);
            const { listeners } = getHandlers(eventType);
            listeners.add(listener);
        },
    });

    Object.defineProperty(globalThis, '__volt_native_event_off__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: (eventType, handler) => {
            const key = normalizeEventType(eventType);
            const listeners = handlers.get(key);
            if (!listeners) {
                return;
            }
            listeners.delete(handler);
            if (listeners.size === 0) {
                handlers.delete(key);
            }
        },
    });

    Object.defineProperty(globalThis, '__volt_native_event_dispatch_safe__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: async (eventType, payload) => {
            const key = normalizeEventType(eventType);
            const listeners = handlers.get(key);
            if (!listeners || listeners.size === 0) {
                return 0;
            }

            const snapshot = Array.from(listeners);
            for (const handler of snapshot) {
                try {
                    await handler(payload);
                } catch (error) {
                    console.error(`[volt] native event handler failed (${key})`, error);
                }
            }
            return snapshot.length;
        },
    });

    Object.defineProperty(globalThis, '__volt_native_event_clear__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: () => handlers.clear(),
    });
})();
"#;

fn format_console_args(args: &[JsValue], context: &mut Context) -> String {
    args.iter()
        .map(|value| {
            super::serde_support::js_value_to_string(context, value.clone())
                .unwrap_or_else(|error| format!("<unprintable: {error}>"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn console_log(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    println!("{message}");
    Ok(JsValue::undefined())
}

fn console_info(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    println!("{message}");
    Ok(JsValue::undefined())
}

fn console_warn(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    eprintln!("{message}");
    Ok(JsValue::undefined())
}

fn console_error(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let message = format_console_args(args, context);
    eprintln!("{message}");
    Ok(JsValue::undefined())
}

fn register_console(context: &mut Context) -> JsResult<()> {
    let console = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(console_log),
            js_string!("log"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_info),
            js_string!("info"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_warn),
            js_string!("warn"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(console_error),
            js_string!("error"),
            1,
        )
        .build();

    context.register_global_property(js_string!("console"), console, Attribute::all())
}

async fn native_sleep(
    _this: &JsValue,
    args: &[JsValue],
    context: &RefCell<&mut Context>,
) -> JsResult<JsValue> {
    let milliseconds = {
        let context = &mut context.borrow_mut();
        let number = args.get_or_undefined(0).to_number(context)?;
        if !number.is_finite() || number <= 0.0 {
            0
        } else if number > u64::MAX as f64 {
            u64::MAX
        } else {
            number.trunc() as u64
        }
    };

    tokio::time::sleep(Duration::from_millis(milliseconds)).await;
    Ok(js_string!(format!("slept:{milliseconds}")).into())
}

fn register_timers(context: &mut Context) -> JsResult<()> {
    context.register_global_builtin_callable(
        js_string!("__volt_native_sleep__"),
        1,
        NativeFunction::from_async_fn(native_sleep),
    )?;
    context
        .eval(Source::from_bytes(TIMER_BOOTSTRAP))
        .map(|_| ())
}

#[cfg(test)]
fn register_native_async_helpers(context: &mut Context) -> JsResult<()> {
    context.register_global_builtin_callable(
        js_string!("nativeSleep"),
        1,
        NativeFunction::from_async_fn(native_sleep),
    )
}

fn register_native_event_bridge_globals(context: &mut Context) -> JsResult<()> {
    context
        .eval(Source::from_bytes(NATIVE_EVENT_BOOTSTRAP))
        .map(|_| ())
}

pub(super) async fn run_jobs(
    context: &mut Context,
    job_executor: &Rc<SimpleJobExecutor>,
) -> Result<(), String> {
    let context_cell = RefCell::new(context);
    job_executor
        .clone()
        .run_jobs_async(&context_cell)
        .await
        .map_err(super::serde_support::js_error)
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
        PromiseState::Rejected(error) => {
            Err(super::serde_support::js_value_to_string(context, error)?)
        }
    }
}

async fn expose_native_modules_on_global(
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
            .map_err(super::serde_support::js_error)?;
    }

    context
        .register_global_property(js_string!("__volt"), volt_modules, Attribute::all())
        .map_err(super::serde_support::js_error)
}

pub(super) async fn initialize_context(
    context: &mut Context,
    module_loader: &MapModuleLoader,
    job_executor: &Rc<SimpleJobExecutor>,
    options: JsRuntimeOptions,
) -> Result<(), String> {
    modules::configure(modules::ModuleConfig {
        fs_base_dir: options.fs_base_dir,
        permissions: options.permissions,
        app_name: options.app_name,
        secure_storage_backend: options.secure_storage_backend,
        updater_telemetry_enabled: options.updater_telemetry_enabled,
        updater_telemetry_sink: options.updater_telemetry_sink,
    })?;

    register_console(context).map_err(super::serde_support::js_error)?;
    register_timers(context).map_err(super::serde_support::js_error)?;
    let registered_modules = modules::register_all_modules(context, module_loader)
        .map_err(super::serde_support::js_error)?;
    expose_native_modules_on_global(context, job_executor, &registered_modules).await?;
    register_native_event_bridge_globals(context).map_err(super::serde_support::js_error)?;
    #[cfg(test)]
    register_native_async_helpers(context).map_err(super::serde_support::js_error)?;
    Ok(())
}
