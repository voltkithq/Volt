import { ipcMain } from 'voltkit';
import type { IpcMessageHandlingOptions, NativeIpcEvent } from '../types.js';
import {
  IPC_IN_FLIGHT_LIMIT_CODE,
  IPC_INVALID_REQUEST_CODE,
  IPC_PAYLOAD_TOO_LARGE_CODE,
} from './constants.js';
import { DEBUG_DEV_IPC, truncateForLog } from './debug.js';
import {
  getWindowIpcInFlightCount,
  releaseWindowIpcSlot,
  tryAcquireWindowIpcSlot,
} from './inflight.js';
import { normalizeNativeIpcRequest } from './request.js';
import {
  createIpcResponseScript,
  extractRequestId,
  measurePayloadBytes,
  normalizeIpcInFlightLimit,
  normalizeIpcPayloadBytes,
  type IpcResponse,
} from './response.js';
import { tryHandleNativeFastPath } from './native-fast-path.js';

export async function handleIpcMessageEvent(
  native: { windowEvalScript(jsId: string, script: string): void },
  event: NativeIpcEvent,
  options: IpcMessageHandlingOptions = {},
): Promise<void> {
  const maxPayloadBytes = normalizeIpcPayloadBytes(options.maxPayloadBytes);
  const payloadBytes = measurePayloadBytes(event.raw);
  if (payloadBytes > maxPayloadBytes) {
    const response: IpcResponse = {
      id: extractRequestId(event.raw),
      error: `IPC payload too large (${payloadBytes} bytes > ${maxPayloadBytes} bytes)`,
      errorCode: IPC_PAYLOAD_TOO_LARGE_CODE,
      errorDetails: { payloadBytes, maxPayloadBytes },
    };
    native.windowEvalScript(event.windowId, createIpcResponseScript(response));
    if (DEBUG_DEV_IPC) {
      console.warn(
        `[volt][ipc] payload-too-large window=${event.windowId} bytes=${payloadBytes} max=${maxPayloadBytes}`,
      );
    }
    return;
  }

  const request = normalizeNativeIpcRequest(event.raw);
  if (!request) {
    const response: IpcResponse = {
      id: extractRequestId(event.raw),
      error: 'Invalid IPC request payload',
      errorCode: IPC_INVALID_REQUEST_CODE,
    };
    native.windowEvalScript(event.windowId, createIpcResponseScript(response));
    if (DEBUG_DEV_IPC) {
      console.warn(
        `[volt][ipc] invalid-request window=${event.windowId} raw=${truncateForLog(JSON.stringify(event.raw))}`,
      );
    }
    return;
  }
  if (DEBUG_DEV_IPC) {
    console.log(
      `[volt][ipc] request id=${request.id} method=${request.method} window=${event.windowId}`,
    );
  }

  const maxInFlightPerWindow = normalizeIpcInFlightLimit(options.maxInFlightPerWindow);
  if (!tryAcquireWindowIpcSlot(event.windowId, maxInFlightPerWindow)) {
    const inFlight = getWindowIpcInFlightCount(event.windowId) ?? maxInFlightPerWindow;
    const response: IpcResponse = {
      id: request.id,
      error: `IPC in-flight limit reached for window ${event.windowId} (${maxInFlightPerWindow})`,
      errorCode: IPC_IN_FLIGHT_LIMIT_CODE,
      errorDetails: {
        windowId: event.windowId,
        maxInFlightPerWindow,
        inFlight,
      },
    };
    native.windowEvalScript(event.windowId, createIpcResponseScript(response));
    if (DEBUG_DEV_IPC) {
      console.warn(
        `[volt][ipc] in-flight-limit window=${event.windowId} method=${request.method} max=${maxInFlightPerWindow}`,
      );
    }
    return;
  }

  let response: IpcResponse;
  try {
    response = await tryHandleNativeFastPath(request) ?? await ipcMain.processRequest(
      request.id,
      request.method,
      request.args,
      { timeoutMs: options.timeoutMs },
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    response = { id: request.id, error: message };
  } finally {
    releaseWindowIpcSlot(event.windowId);
  }

  native.windowEvalScript(event.windowId, createIpcResponseScript(response));
  if (DEBUG_DEV_IPC) {
    console.log(
      `[volt][ipc] response id=${request.id} method=${request.method} status=${response.error ? 'error' : 'ok'}`
      + `${response.error ? ` error=${truncateForLog(response.error)}` : ''}`,
    );
  }
}
