import { randomUUID } from 'node:crypto';
import { resolve } from 'node:path';
import { loadConfig } from '../utils/config.js';
import { createApp } from 'voltkit';
import { __internalDispatchMenuEvent } from 'voltkit/internal';
import { loadBackendEntrypointForDev } from './dev/backend.js';
import type { NativeHostWindowConfig } from './native-host-protocol.js';
import {
  clearIpcLoadState,
  createIpcResponseScript,
  handleIpcMessageEvent,
  handleWindowClosedEventForIpcState,
  isNativeIpcRequest,
  parseNativeEvent,
  resetIpcRuntimeState,
  syncFrameworkWindowStateFromNativeCloseEvent,
} from './dev/ipc.js';
import { configureRuntimeModuleState, resetRuntimeModuleState } from './dev/runtime-modules/shared.js';
import {
  currentRuntimeMode,
  isHostToParentMessage,
  loadNativeBinding,
  runtimeModeForPlatform,
  shouldUseOutOfProcessNativeHost,
  startNativeRuntime,
  startOutOfProcessRuntime,
} from './dev/runtime.js';
import { parseDevPort, resolveViteDevUrl, spawnVite } from './dev/server.js';
import { startPluginDevelopment } from './dev/plugin-development.js';

export interface DevOptions {
  port: string;
  host: string;
}

function isTruthyEnv(value: string | undefined): boolean {
  if (!value) {
    return false;
  }
  const normalized = value.trim().toLowerCase();
  return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on';
}

/**
 * Start the Vite dev server and open a native window pointing to it.
 *
 * Architecture: Vite runs as a separate `npx vite` child process. The native
 * runtime runs independently and callbacks are bridged back through N-API.
 */
export async function devCommand(options: DevOptions): Promise<void> {
  const cwd = process.cwd();
  const port = parseDevPort(options.port);
  const debugIpc = isTruthyEnv(process.env.VOLT_DEBUG_IPC);

  console.log('[volt] Starting development server...');

  const config = await loadConfig(cwd);
  console.log(`[volt] App: ${config.name}`);
  console.log(`[volt] Runtime mode: ${currentRuntimeMode()}`);

  const devHost = options.host === '0.0.0.0' ? 'localhost' : options.host;
  const requestedDevUrl = `http://${devHost}:${port}`;

  const vite = spawnVite(cwd, port, options.host);

  console.log('[volt] Waiting for Vite server...');
  const devUrl = await resolveViteDevUrl(vite, requestedDevUrl, 15000);
  if (!devUrl) {
    console.error('[volt] Vite server failed to start within 15s.');
    console.log(`[volt] Check if port ${port} is in use.`);
    vite.child.kill();
    process.exit(1);
  }

  console.log(`[volt] Vite dev server ready at ${devUrl}`);

  let shutdownNativeRuntime = () => {};
  let disposeBackendBundle = () => {};
  let disposePluginDevelopment = async () => {};
  let nativeRuntimeExitPromise: Promise<void> | null = null;
  let cleanupStarted = false;
  const waitFor = (timeoutMs: number) =>
    new Promise<void>((resolve) => {
      const timer = setTimeout(resolve, timeoutMs);
      timer.unref();
    });

  let onSignal = () => {};
  const gracefulCleanup = async (exitCode: number | null): Promise<void> => {
    if (cleanupStarted) {
      return;
    }
    cleanupStarted = true;
    process.off('SIGINT', onSignal);
    process.off('SIGTERM', onSignal);
    resetIpcRuntimeState();
    resetRuntimeModuleState();

    try {
      disposeBackendBundle();
    } catch {
      // Continue cleanup even if temporary backend bundle cleanup fails.
    }

    await disposePluginDevelopment().catch(() => {});

    try {
      shutdownNativeRuntime();
    } catch {
      // Continue cleanup even if shutdown signaling fails.
    }

    if (nativeRuntimeExitPromise) {
      await Promise.race([nativeRuntimeExitPromise.catch(() => {}), waitFor(2500)]);
    }

    vite.child.kill();
    if (exitCode !== null) {
      process.exit(exitCode);
    }
  };

  const native = loadNativeBinding();
  if (!native) {
    console.log('[volt] Native binding not available. Running in web-only mode.');
    console.log(`[volt] Open ${devUrl} in your browser.`);
    console.log('[volt] Press Ctrl+C to stop.');
    await new Promise<void>((resolve) => {
      process.on('SIGINT', resolve);
    });
    vite.child.kill();
    return;
  }

  const windowConfig: NativeHostWindowConfig = {
    name: config.name,
    permissions: config.permissions ?? [],
    jsId: randomUUID(),
    url: devUrl,
    devtools: config.devtools ?? true,
    window: {
      title: config.window?.title ?? config.name,
      width: config.window?.width ?? 800,
      height: config.window?.height ?? 600,
      minWidth: config.window?.minWidth,
      minHeight: config.window?.minHeight,
      resizable: config.window?.resizable ?? true,
      decorations: config.window?.decorations ?? true,
      icon: config.window?.icon ? resolve(cwd, config.window.icon) : undefined,
    },
  };

  const frameworkApp = createApp({
    name: config.name,
    version: config.version,
    devtools: config.devtools ?? true,
    window: config.window,
    permissions: config.permissions,
  });

  console.log('[volt] Creating native window...');
  let nativeRuntime: Awaited<ReturnType<typeof startNativeRuntime>>;
  try {
    nativeRuntime = await startNativeRuntime(native, windowConfig);
  } catch (err) {
    await gracefulCleanup(null);
    throw err;
  }

  onSignal = () => {
    void gracefulCleanup(0);
  };
  process.on('SIGINT', onSignal);
  process.on('SIGTERM', onSignal);

  frameworkApp.setNativeApp(nativeRuntime as unknown);

  configureRuntimeModuleState({
    projectRoot: cwd,
    defaultWindowId: windowConfig.jsId,
    nativeRuntime,
  });

  let backendLoadState: Awaited<ReturnType<typeof loadBackendEntrypointForDev>>;
  try {
    backendLoadState = await loadBackendEntrypointForDev(cwd, config.backend);
  } catch (err) {
    await gracefulCleanup(null);
    const message = err instanceof Error ? err.message : String(err);
    throw new Error(`[volt] Failed to load backend entry for dev mode: ${message}`, { cause: err });
  }

  disposeBackendBundle = backendLoadState.dispose;
  if (backendLoadState.loaded) {
    console.log(`[volt] Backend loaded from ${backendLoadState.backendEntryPath}`);
    try {
      const stopWatching = await backendLoadState.watch((ok) => {
        if (ok) {
          console.log('[volt] Backend changes applied.');
        }
      });
      const originalDispose = disposeBackendBundle;
      disposeBackendBundle = () => {
        stopWatching();
        originalDispose();
      };
    } catch {
      console.warn('[volt] Backend file watching unavailable.');
    }
  } else {
    console.log('[volt] No backend entry found for dev mode.');
  }

  shutdownNativeRuntime = () => nativeRuntime.shutdown();
  disposePluginDevelopment = await startPluginDevelopment(cwd, config.plugins);

  // Register the event handler BEFORE run() so no IPC messages are dropped.
  // run() starts the native event loop and the WebView becomes live immediately,
  // so the listener must already be in place to handle incoming IPC calls.
  nativeRuntime.onEvent((eventJson: string) => {
    if (debugIpc) {
      console.log(`[volt][native-event] ${eventJson}`);
    }
    try {
      const parsed = parseNativeEvent(JSON.parse(eventJson));
      if (!parsed) {
        if (debugIpc) {
          console.warn('[volt][native-event] unrecognized event payload');
        }
        return;
      }

      if (debugIpc) {
        console.log(`[volt][native-event] parsed type=${parsed.type}`);
      }

      if (parsed.type === 'ipc-message') {
        void handleIpcMessageEvent(nativeRuntime, parsed, { timeoutMs: 30_000 }).catch((err) => {
          const message = err instanceof Error ? err.message : String(err);
          console.error(`[volt] Failed to process IPC message: ${message}`);
        });
        return;
      }

      if (parsed.type === 'quit') {
        console.log('[volt] Window closed.');
        void gracefulCleanup(0);
        return;
      }

      if (parsed.type === 'window-closed') {
        syncFrameworkWindowStateFromNativeCloseEvent(parsed);
        return;
      }

      if (parsed.type === 'menu-event') {
        __internalDispatchMenuEvent(parsed.menuId);
      }
    } catch (error) {
      if (debugIpc) {
        const message = error instanceof Error ? error.message : String(error);
        console.warn(`[volt][native-event] failed to parse event: ${message}`);
      }
    }
  });

  nativeRuntimeExitPromise = nativeRuntime.run();

  frameworkApp.markReady();

  console.log('[volt] Window open. Press F12 for DevTools.');

  try {
    await nativeRuntimeExitPromise;
    await gracefulCleanup(null);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error(`[volt] Native runtime exited unexpectedly: ${message}`);
    await gracefulCleanup(1);
  }
}

export const __testOnly = {
  handleIpcMessageEvent,
  isNativeIpcRequest,
  createIpcResponseScript,
  parseNativeEvent,
  isHostToParentMessage,
  startOutOfProcessRuntime,
  runtimeModeForPlatform,
  currentRuntimeMode,
  shouldUseOutOfProcessNativeHost,
  parseDevPort,
  handleWindowClosedEventForIpcState,
  syncFrameworkWindowStateFromNativeCloseEvent,
  clearIpcLoadState,
  resetIpcRuntimeState,
};
