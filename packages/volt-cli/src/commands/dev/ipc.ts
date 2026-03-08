import { BrowserWindow, ipcMain } from 'voltkit';
import type { NativeWindowClosedEvent, ResolveWindowByJsId } from './types.js';
import { parseNativeEvent } from './ipc/event-parser.js';
import { handleIpcMessageEvent } from './ipc/handler.js';
import { clearIpcLoadState, handleWindowClosedEventForIpcState } from './ipc/inflight.js';
import { isNativeIpcRequest } from './ipc/request.js';
import { createIpcResponseScript } from './ipc/response.js';

export {
  clearIpcLoadState,
  createIpcResponseScript,
  handleIpcMessageEvent,
  handleWindowClosedEventForIpcState,
  isNativeIpcRequest,
  parseNativeEvent,
};

function clearIpcHandlers(): void {
  const maybeIpcMain = ipcMain as unknown as { clearHandlers?: () => void };
  maybeIpcMain.clearHandlers?.();
}

export function syncFrameworkWindowStateFromNativeCloseEvent(
  event: NativeWindowClosedEvent,
  resolveWindowByJsId: ResolveWindowByJsId = (jsWindowId) => BrowserWindow.fromId(jsWindowId),
): void {
  handleWindowClosedEventForIpcState(event);
  if (!event.jsWindowId) {
    return;
  }
  resolveWindowByJsId(event.jsWindowId)?.destroy();
}

export function resetIpcRuntimeState(): void {
  clearIpcHandlers();
  clearIpcLoadState();
}
