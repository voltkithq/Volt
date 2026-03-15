use boa_engine::module::{Module, SyntheticModuleInitializer};
use boa_engine::{Context, JsResult, JsValue, Source, js_string};

pub(super) const VOLT_PLUGIN_BOOTSTRAP: &str = r#"
(() => {
    const state = {
        activate: null,
        deactivate: null,
        commands: new Map(),
        ipc: new Map(),
        events: new Map(),
        grants: new Map(),
        context: null,
    };

    const native = globalThis.__volt_plugin_native__;
    if (!native) {
        throw new Error('volt:plugin native bridge is unavailable');
    }

    const freezeDeep = (value) => {
        if (!value || typeof value !== 'object' || Object.isFrozen(value)) {
            return value;
        }
        Object.freeze(value);
        for (const nested of Object.values(value)) {
            freezeDeep(nested);
        }
        return value;
    };

    const ensureName = (value, label) => {
        if (typeof value !== 'string') {
            throw new TypeError(`${label} must be a string`);
        }
        const normalized = value.trim();
        if (!normalized) {
            throw new TypeError(`${label} must not be empty`);
        }
        return normalized;
    };

    const ensureHandler = (value, label) => {
        if (typeof value !== 'function') {
            throw new TypeError(`${label} must be a function`);
        }
        return value;
    };

    const ensureObject = (value, label) => {
        if (value === undefined) {
            return {};
        }
        if (!value || typeof value !== 'object' || Array.isArray(value)) {
            throw new TypeError(`${label} must be an object`);
        }
        return value;
    };

    const dispatchEvent = async (event, data) => {
        const handlers = state.events.get(ensureName(event, 'event name')) ?? [];
        for (const handler of handlers) {
            await handler(data ?? null);
        }
        return null;
    };

    const rememberGrant = (grant) => {
        if (grant && grant.grantId) {
            state.grants.set(grant.grantId, Object.freeze(grant));
        }
        return grant;
    };

    for (const grant of native.delegatedGrants()) {
        rememberGrant(grant);
    }

    const assertGrantActive = (grantId) => {
        if (!state.grants.has(grantId)) {
            throw new Error(`grant is not delegated to this plugin: ${grantId}`);
        }
    };

    const createGrantFs = (grantId) => Object.freeze({
        grantId,
        readFile(path) {
            assertGrantActive(grantId);
            return native.grantFsReadFile(grantId, ensureName(path, 'path'));
        },
        writeFile(path, data) {
            assertGrantActive(grantId);
            native.grantFsWriteFile(grantId, ensureName(path, 'path'), String(data));
        },
        readDir(path) {
            assertGrantActive(grantId);
            return native.grantFsReadDir(grantId, ensureName(path, 'path'));
        },
        stat(path) {
            assertGrantActive(grantId);
            return native.grantFsStat(grantId, ensureName(path, 'path'));
        },
        exists(path) {
            assertGrantActive(grantId);
            return native.grantFsExists(grantId, ensureName(path, 'path'));
        },
        mkdir(path) {
            assertGrantActive(grantId);
            return native.grantFsMkdir(grantId, ensureName(path, 'path'));
        },
        remove(path) {
            assertGrantActive(grantId);
            return native.grantFsRemove(grantId, ensureName(path, 'path'));
        },
    });

    const context = Object.freeze({
        manifest: freezeDeep(native.manifest()),
        log: Object.freeze({
            info(message) { native.sendLog('info', String(message)); },
            warn(message) { native.sendLog('warn', String(message)); },
            error(message) { native.sendLog('error', String(message)); },
            debug(message) { native.sendLog('debug', String(message)); },
        }),
        commands: Object.freeze({
            register(id, handler) {
                const commandId = ensureName(id, 'command id');
                ensureHandler(handler, 'command handler');
                state.commands.set(commandId, handler);
                native.registerCommand(commandId);
            },
            unregister(id) {
                const commandId = ensureName(id, 'command id');
                state.commands.delete(commandId);
                native.unregisterCommand(commandId);
            },
        }),
        events: Object.freeze({
            on(event, handler) {
                const eventName = ensureName(event, 'event name');
                const listener = ensureHandler(handler, 'event handler');
                const handlers = state.events.get(eventName) ?? [];
                handlers.push(listener);
                state.events.set(eventName, handlers);
                if (handlers.length === 1) {
                    native.subscribeEvent(eventName);
                }
            },
            off(event, handler) {
                const eventName = ensureName(event, 'event name');
                const handlers = state.events.get(eventName);
                if (!handlers) {
                    return;
                }
                const nextHandlers = handlers.filter((candidate) => candidate !== handler);
                if (nextHandlers.length === 0) {
                    state.events.delete(eventName);
                    native.unsubscribeEvent(eventName);
                    return;
                }
                state.events.set(eventName, nextHandlers);
            },
            emit(event, data) {
                native.emitEvent(ensureName(event, 'event name'), data ?? null);
            },
        }),
        ipc: Object.freeze({
            handle(channel, handler) {
                const ipcChannel = ensureName(channel, 'ipc channel');
                ensureHandler(handler, 'ipc handler');
                state.ipc.set(ipcChannel, handler);
                native.registerIpc(ipcChannel);
            },
            removeHandler(channel) {
                const ipcChannel = ensureName(channel, 'ipc channel');
                state.ipc.delete(ipcChannel);
                native.unregisterIpc(ipcChannel);
            },
        }),
        fs: Object.freeze({
            readFile(path) { return native.fsReadFile(ensureName(path, 'path')); },
            writeFile(path, data) { native.fsWriteFile(ensureName(path, 'path'), String(data)); },
            readDir(path) { return native.fsReadDir(ensureName(path, 'path')); },
            stat(path) { return native.fsStat(ensureName(path, 'path')); },
            exists(path) { return native.fsExists(ensureName(path, 'path')); },
            mkdir(path) { return native.fsMkdir(ensureName(path, 'path')); },
            remove(path) { return native.fsRemove(ensureName(path, 'path')); },
        }),
        storage: Object.freeze({
            async get(key) { return native.storageGet(ensureName(key, 'storage key')); },
            async set(key, value) {
                native.storageSet(ensureName(key, 'storage key'), String(value));
            },
            async has(key) { return native.storageHas(ensureName(key, 'storage key')); },
            async delete(key) { return native.storageDelete(ensureName(key, 'storage key')); },
            async keys() { return native.storageKeys(); },
        }),
        grants: Object.freeze({
            async requestAccess(options) {
                return rememberGrant(
                    native.requestAccess(ensureObject(options, 'request access options'))
                );
            },
            async list() {
                return native.listGrants();
            },
            bindFsScope(grantId) {
                const normalizedGrantId = ensureName(grantId, 'grant id');
                rememberGrant(native.bindGrant(normalizedGrantId));
                return createGrantFs(normalizedGrantId);
            },
        }),
    });
    state.context = context;

    const maybeAwait = async (value) => await value;

    const definePlugin = (config) => {
        if (!config || typeof config !== 'object') {
            throw new TypeError('definePlugin config must be an object');
        }
        if (config.activate !== undefined && typeof config.activate !== 'function') {
            throw new TypeError('activate must be a function');
        }
        if (config.deactivate !== undefined && typeof config.deactivate !== 'function') {
            throw new TypeError('deactivate must be a function');
        }
        state.activate = config.activate ?? null;
        state.deactivate = config.deactivate ?? null;
    };

    globalThis.__volt_plugin_activate__ = async () => {
        if (state.activate) {
            await maybeAwait(state.activate(state.context));
        }
        return null;
    };

    globalThis.__volt_plugin_deactivate__ = async () => {
        if (state.deactivate) {
            await maybeAwait(state.deactivate(state.context));
        }
        return null;
    };

    globalThis.__volt_plugin_invoke_command__ = async (id, args) => {
        const handler = state.commands.get(ensureName(id, 'command id'));
        if (!handler) {
            throw new Error(`command handler not found: ${id}`);
        }
        return await maybeAwait(handler(args ?? null));
    };

    globalThis.__volt_plugin_invoke_ipc__ = async (channel, args) => {
        const handler = state.ipc.get(ensureName(channel, 'ipc channel'));
        if (!handler) {
            throw new Error(`ipc handler not found: ${channel}`);
        }
        return await maybeAwait(handler(args ?? null));
    };

    globalThis.__volt_plugin_dispatch_event__ = async (event, data) => {
        return dispatchEvent(event, data);
    };

    globalThis.__volt_plugin_revoke_grant__ = async (grantId) => {
        const normalizedGrantId = ensureName(grantId, 'grant id');
        state.grants.delete(normalizedGrantId);
        await dispatchEvent('grant:revoked', { grantId: normalizedGrantId });
        return null;
    };

    return definePlugin;
})()
"#;

pub fn build_module(context: &mut Context) -> JsResult<Module> {
    let define_plugin = context.eval(Source::from_bytes(VOLT_PLUGIN_BOOTSTRAP))?;

    Ok(Module::synthetic(
        &[js_string!("definePlugin")],
        SyntheticModuleInitializer::from_copy_closure_with_captures(
            |module, define_plugin: &JsValue, _context| {
                module.set_export(&js_string!("definePlugin"), define_plugin.clone())?;
                Ok(())
            },
            define_plugin,
        ),
        None,
        None,
        context,
    ))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
