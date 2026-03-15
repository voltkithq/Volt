use boa_engine::{Context, JsResult, Source};

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

pub(super) fn register_native_event_bridge_globals(context: &mut Context) -> JsResult<()> {
    context
        .eval(Source::from_bytes(NATIVE_EVENT_BOOTSTRAP))
        .map(|_| ())
}
