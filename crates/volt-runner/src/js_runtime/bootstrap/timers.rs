use std::cell::RefCell;
use std::time::Duration;

use boa_engine::native_function::NativeFunction;
use boa_engine::{Context, JsArgs, JsResult, JsValue, Source, js_string};

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

pub(super) fn register_timers(context: &mut Context) -> JsResult<()> {
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
pub(super) fn register_native_async_helpers(context: &mut Context) -> JsResult<()> {
    context.register_global_builtin_callable(
        js_string!("nativeSleep"),
        1,
        NativeFunction::from_async_fn(native_sleep),
    )
}
