use boa_engine::module::{Module, SyntheticModuleInitializer};
use boa_engine::{Context, JsResult, JsValue, Source, js_string};

// DEPENDENCY: This module requires volt:events to be loaded on globalThis.__volt.events
// before IPC handlers can use emit/emitTo. The module loading order in bootstrap.rs
// guarantees this: events is registered before ipc.
const IPC_MODULE_BOOTSTRAP: &str = r#"
(() => {
    const handlers = new Map();

    const normalizeMethod = (method) => {
        if (typeof method !== 'string') {
            throw new TypeError('IPC method must be a string');
        }
        const normalized = method.trim();
        if (!normalized) {
            throw new TypeError('IPC method must not be empty');
        }
        return normalized;
    };

    const ensureHandler = (handler) => {
        if (typeof handler !== 'function') {
            throw new TypeError('IPC handler must be a function');
        }
        return handler;
    };

    const ensureUserChannel = (method) => {
        if (
            method.startsWith('volt:')
            || method.startsWith('__volt_internal:')
            || method.startsWith('plugin:')
        ) {
            throw new Error(`IPC channel is reserved by Volt: ${method}`);
        }
        return method;
    };

    const toErrorPayload = (error) => {
        const message = error instanceof Error ? error.message : String(error);
        let errorCode = 'IPC_HANDLER_ERROR';
        let errorDetails = null;

        if (error && typeof error === 'object') {
            if (typeof error.code === 'string' && error.code.length > 0) {
                errorCode = error.code;
            }
            if (Object.prototype.hasOwnProperty.call(error, 'details')) {
                errorDetails = error.details;
            } else if (Object.prototype.hasOwnProperty.call(error, 'errorDetails')) {
                errorDetails = error.errorDetails;
            }
        }

        return {
            error: message,
            errorCode,
            errorDetails,
        };
    };

    const ipcMain = Object.freeze({
        handle(method, handler) {
            const key = ensureUserChannel(normalizeMethod(method));
            ensureHandler(handler);
            if (handlers.has(key)) {
                throw new Error(`IPC handler already registered for channel: ${key}`);
            }
            handlers.set(key, handler);
        },
        removeHandler(method) {
            handlers.delete(normalizeMethod(method));
        },
        clearHandlers() {
            handlers.clear();
        },
        hasHandler(method) {
            return handlers.has(normalizeMethod(method));
        },
        emit(eventName, data) {
            if (
                !globalThis.__volt
                || !globalThis.__volt.events
                || typeof globalThis.__volt.events.emit !== 'function'
            ) {
                throw new Error('volt:events module is unavailable');
            }
            globalThis.__volt.events.emit(eventName, data);
        },
        emitTo(windowId, eventName, data) {
            if (
                !globalThis.__volt
                || !globalThis.__volt.events
                || typeof globalThis.__volt.events.emitTo !== 'function'
            ) {
                throw new Error('volt:events module is unavailable');
            }
            globalThis.__volt.events.emitTo(windowId, eventName, data);
        },
    });

    Object.defineProperty(globalThis, '__volt_ipc_dispatch_safe__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: async (method, args) => {
            try {
                const key = normalizeMethod(method);
                const handler = handlers.get(key);
                if (!handler) {
                    const error = new Error(`Handler not found: ${key}`);
                    error.code = 'IPC_HANDLER_NOT_FOUND';
                    throw error;
                }
                const result = await handler(args);
                return {
                    ok: true,
                    result: result === undefined ? null : result,
                };
            } catch (error) {
                const payload = toErrorPayload(error);
                return {
                    ok: false,
                    error: payload.error,
                    errorCode: payload.errorCode,
                    errorDetails: payload.errorDetails,
                };
            }
        },
    });

    Object.defineProperty(globalThis, '__volt_ipc_reset__', {
        configurable: false,
        enumerable: false,
        writable: false,
        value: () => {
            handlers.clear();
        },
    });

    return ipcMain;
})()
"#;

pub fn build_module(context: &mut Context) -> JsResult<Module> {
    let ipc_main = context.eval(Source::from_bytes(IPC_MODULE_BOOTSTRAP))?;

    Ok(Module::synthetic(
        &[js_string!("ipcMain")],
        SyntheticModuleInitializer::from_copy_closure_with_captures(
            |module, ipc_main: &JsValue, _context| {
                module.set_export(&js_string!("ipcMain"), ipc_main.clone())?;
                Ok(())
            },
            ipc_main,
        ),
        None,
        None,
        context,
    ))
}
