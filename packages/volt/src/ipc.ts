/**
 * IPC (Inter-Process Communication) module.
 * Provides the `ipcMain` API for the backend (Node.js) side.
 */

type IpcHandler = (args: unknown) => Promise<unknown> | unknown;

const handlers = new Map<string, IpcHandler>();
const RESERVED_IPC_PREFIXES = ['volt:', '__volt_internal:', 'plugin:'];

export type IpcErrorCode =
  | 'IPC_HANDLER_NOT_FOUND'
  | 'IPC_HANDLER_ERROR'
  | 'IPC_HANDLER_TIMEOUT'
  | 'IPC_PAYLOAD_TOO_LARGE'
  | 'IPC_IN_FLIGHT_LIMIT';

export interface IpcProcessResponse {
  id: string;
  result?: unknown;
  error?: string;
  errorCode?: IpcErrorCode;
  errorDetails?: unknown;
}

export interface IpcProcessOptions {
  timeoutMs?: number;
}

const DEFAULT_HANDLER_TIMEOUT_MS = 5000;

/**
 * Backend IPC API (Electron-compatible pattern).
 * Used in the main process to register handlers that the renderer can invoke.
 */
export const ipcMain = {
  /**
   * Register a handler for an IPC channel.
   * The handler receives arguments from the renderer and returns a result.
   *
   * @example
   * ```ts
   * ipcMain.handle('get-user', async (args) => {
   *   return { name: 'Alice', id: args.id };
   * });
   * ```
   */
  handle(channel: string, handler: IpcHandler): void {
    assertNonReservedChannel(channel);
    if (handlers.has(channel)) {
      throw new Error(`IPC handler already registered for channel: ${channel}`);
    }
    handlers.set(channel, handler);
  },

  /**
   * Remove a previously registered handler.
   */
  removeHandler(channel: string): void {
    handlers.delete(channel);
  },

  /**
   * Remove all registered handlers.
   * @internal
   */
  clearHandlers(): void {
    handlers.clear();
  },

  /**
   * Get a registered handler (internal use).
   * @internal
   */
  getHandler(channel: string): IpcHandler | undefined {
    return handlers.get(channel);
  },

  /**
   * Check if a handler is registered for a channel.
   */
  hasHandler(channel: string): boolean {
    return handlers.has(channel);
  },

  /**
   * Process an IPC request from the renderer.
   * @internal
   */
  async processRequest(
    id: string,
    method: string,
    args: unknown,
    options: IpcProcessOptions = {},
  ): Promise<IpcProcessResponse> {
    const handler = handlers.get(method);
    if (!handler) {
      return {
        id,
        error: `Handler not found: ${method}`,
        errorCode: 'IPC_HANDLER_NOT_FOUND',
      };
    }

    const timeoutMs = normalizeTimeout(options.timeoutMs);

    try {
      const result = await withTimeout(
        Promise.resolve(handler(args)),
        timeoutMs,
        `IPC handler timed out after ${timeoutMs}ms: ${method}`,
      );
      return { id, result: result ?? null };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (isTimeoutError(err)) {
        return {
          id,
          error: message,
          errorCode: 'IPC_HANDLER_TIMEOUT',
          errorDetails: { timeoutMs, method },
        };
      }
      return {
        id,
        error: message,
        errorCode: 'IPC_HANDLER_ERROR',
      };
    }
  },
};

/**
 * Renderer-side IPC API.
 * These functions are available in the WebView context via `window.__volt__`.
 * This module provides TypeScript types for the renderer API.
 */

/**
 * Invoke an IPC handler registered in the main process.
 * This is the renderer-side API - it calls through to the Rust IPC bridge.
 *
 * @example
 * ```ts
 * // In the renderer (browser context)
 * const user = await invoke<User>('get-user', { id: 1 });
 * ```
 */
export function invoke<T = unknown>(
  channel: string,
  ...args: unknown[]
): Promise<T> {
  // This function is meant to be used in the renderer context
  // where window.__volt__ is available. In Node.js context, throw.
  const volt = getVoltBridge();
  if (volt) {
    let payload: unknown = null;
    if (args.length === 1) {
      payload = args[0];
    } else if (args.length > 1) {
      payload = args;
    }
    return volt.invoke(channel, payload) as Promise<T>;
  }
  throw new Error(
    'invoke() can only be called from the renderer (WebView) context.',
  );
}

/**
 * Listen for events emitted from the main process.
 * This is the renderer-side API.
 */
export function on(event: string, callback: (...args: unknown[]) => void): void {
  const volt = getVoltBridge();
  if (volt) {
    volt.on(event, callback);
    return;
  }
  throw new Error(
    'on() can only be called from the renderer (WebView) context.',
  );
}

/**
 * Remove an event listener.
 * This is the renderer-side API.
 */
export function off(event: string, callback: (...args: unknown[]) => void): void {
  const volt = getVoltBridge();
  if (volt) {
    volt.off(event, callback);
    return;
  }
  throw new Error(
    'off() can only be called from the renderer (WebView) context.',
  );
}

interface VoltBridge {
  invoke(method: string, args: unknown): Promise<unknown>;
  on(event: string, callback: (...args: unknown[]) => void): void;
  off(event: string, callback: (...args: unknown[]) => void): void;
}

function normalizeTimeout(timeoutMs?: number): number {
  if (typeof timeoutMs !== 'number' || !Number.isFinite(timeoutMs)) {
    return DEFAULT_HANDLER_TIMEOUT_MS;
  }
  if (timeoutMs <= 0) {
    return DEFAULT_HANDLER_TIMEOUT_MS;
  }
  return timeoutMs;
}

function assertNonReservedChannel(channel: string): void {
  if (typeof channel !== 'string') {
    throw new Error('IPC handler channel must be a string');
  }
  if (RESERVED_IPC_PREFIXES.some((prefix) => channel.trim().startsWith(prefix))) {
    throw new Error(`IPC channel is reserved by Volt: ${channel.trim()}`);
  }
}

function isTimeoutError(err: unknown): boolean {
  if (!(err instanceof Error)) {
    return false;
  }
  return err.name === 'TimeoutError';
}

function timeoutError(message: string): Error {
  const err = new Error(message);
  err.name = 'TimeoutError';
  return err;
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number, message: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(timeoutError(message));
    }, timeoutMs);
    if (typeof (timer as { unref?: () => void }).unref === 'function') {
      (timer as { unref: () => void }).unref();
    }

    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}

/** Get the Volt IPC bridge from the window global, or null in Node.js. */
function getVoltBridge(): VoltBridge | null {
  // Use globalThis to access window in a way that works in both Node.js and browser.
  // In Node.js, globalThis.window is undefined. In browser, it's the Window object.
  const g = globalThis as Record<string, unknown>;
  if (typeof g['window'] !== 'undefined') {
    const w = g['window'] as Record<string, unknown>;
    if (w['__volt__']) {
      return w['__volt__'] as VoltBridge;
    }
  }
  return null;
}
