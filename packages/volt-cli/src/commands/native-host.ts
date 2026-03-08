import { createRequire } from 'node:module';
import {
  NATIVE_HOST_PROTOCOL_VERSION,
  type NativeHostWindowConfig,
  type HostToParentMessage,
  type ParentToHostMessage,
} from './native-host-protocol.js';
import { extractNativeEventJson } from './dev/runtime-event.js';

interface NativeBinding {
  VoltApp: new (config: NativeHostWindowConfig) => {
    createWindow(config: NativeHostWindowConfig): void;
    onEvent(callback: (...args: unknown[]) => void): void;
    run(): void;
  };
  windowEvalScript(jsId: string, script: string): void;
  windowClose(jsId: string): void;
  windowShow(jsId: string): void;
  windowFocus(jsId: string): void;
  windowMaximize(jsId: string): void;
  windowMinimize(jsId: string): void;
  windowRestore(jsId: string): void;
}

let native: NativeBinding | null = null;
let nativeApp: InstanceType<NativeBinding['VoltApp']> | null = null;
let primaryWindowId: string | null = null;
let started = false;
let shuttingDown = false;
let exited = false;

function send(message: HostToParentMessage): void {
  if (typeof process.send === 'function') {
    process.send(message);
  }
}

function exitHost(code: number): void {
  if (exited) {
    return;
  }

  exited = true;
  send({
    type: 'stopped',
    protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
    code,
  });
  process.exit(code);
}

function loadNativeBinding(): NativeBinding | null {
  try {
    const require = createRequire(import.meta.url);
    return require('@voltkit/volt-native') as NativeBinding;
  } catch {
    return null;
  }
}

function startRuntime(windowConfig: NativeHostWindowConfig): void {
  if (started) {
    return;
  }
  started = true;
  shuttingDown = false;
  send({
    type: 'starting',
    protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
  });
  primaryWindowId = windowConfig.jsId;

  native = loadNativeBinding();
  if (!native) {
    send({
      type: 'native-unavailable',
      protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
      message: 'Native binding @voltkit/volt-native is not available',
    });
    exitHost(1);
    return;
  }

  try {
    nativeApp = new native.VoltApp(windowConfig);
    nativeApp.createWindow(windowConfig);
    nativeApp.onEvent((...callbackArgs: unknown[]) => {
      const eventJson = extractNativeEventJson(callbackArgs);
      if (!eventJson) {
        return;
      }
      send({
        type: 'event',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        eventJson,
      });
    });
    send({
      type: 'ready',
      protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
    });
    nativeApp.run();
    // In split-runtime mode, run() can return after spawning the native loop.
    // Keep the host alive until explicit shutdown/disconnect.
    if (shuttingDown) {
      exitHost(0);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    send({
      type: 'runtime-error',
      protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
      message,
    });
    exitHost(1);
  }
}

function evalScript(jsId: string, script: string): void {
  if (!native) {
    return;
  }
  native.windowEvalScript(jsId, script);
}

function dispatchWindowCommand(
  command:
    | 'close-window'
    | 'show-window'
    | 'focus-window'
    | 'maximize-window'
    | 'minimize-window'
    | 'restore-window',
  jsId: string,
): void {
  if (!native) {
    return;
  }
  switch (command) {
    case 'close-window':
      native.windowClose(jsId);
      break;
    case 'show-window':
      native.windowShow(jsId);
      break;
    case 'focus-window':
      native.windowFocus(jsId);
      break;
    case 'maximize-window':
      native.windowMaximize(jsId);
      break;
    case 'minimize-window':
      native.windowMinimize(jsId);
      break;
    case 'restore-window':
      native.windowRestore(jsId);
      break;
    default:
      break;
  }
}

function shutdown(): void {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  send({
    type: 'stopping',
    protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
  });
  if (native && primaryWindowId) {
    try {
      native.windowClose(primaryWindowId);
    } catch {
      // Best-effort shutdown; force exit fallback below.
    }
  }
  setTimeout(() => exitHost(0), 2000).unref();
}

function onMessage(message: unknown): void {
  if (!isParentToHostMessage(message)) {
    return;
  }

  if (message.protocolVersion !== NATIVE_HOST_PROTOCOL_VERSION) {
    send({
      type: 'runtime-error',
      protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
      message: `Unsupported protocol version: ${String(message.protocolVersion)}`,
    });
    exitHost(1);
    return;
  }

  switch (message.type) {
    case 'start':
      startRuntime(message.windowConfig);
      break;
    case 'eval-script':
      evalScript(message.jsId, message.script);
      break;
    case 'close-window':
    case 'show-window':
    case 'focus-window':
    case 'maximize-window':
    case 'minimize-window':
    case 'restore-window':
      dispatchWindowCommand(message.type, message.jsId);
      break;
    case 'shutdown':
      shutdown();
      break;
    case 'ping':
      send({
        type: 'pong',
        protocolVersion: NATIVE_HOST_PROTOCOL_VERSION,
        pingId: message.pingId,
      });
      break;
    default:
      break;
  }
}

function isParentToHostMessage(value: unknown): value is ParentToHostMessage {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const message = value as Record<string, unknown>;
  if (typeof message.type !== 'string') {
    return false;
  }
  if (typeof message.protocolVersion !== 'number') {
    return false;
  }

  switch (message.type) {
    case 'start':
      return !!message.windowConfig && typeof message.windowConfig === 'object';
    case 'eval-script':
      return typeof message.jsId === 'string' && typeof message.script === 'string';
    case 'close-window':
    case 'show-window':
    case 'focus-window':
    case 'maximize-window':
    case 'minimize-window':
    case 'restore-window':
      return typeof message.jsId === 'string';
    case 'shutdown':
      return true;
    case 'ping':
      return typeof message.pingId === 'number';
    default:
      return false;
  }
}

process.on('message', onMessage);
process.on('disconnect', shutdown);
