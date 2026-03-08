import { ipcMain as frameworkIpcMain } from 'voltkit';
import { emitFrontendEvent } from './shared.js';

type FrameworkIpcMain = typeof frameworkIpcMain;

type DevIpcMain = FrameworkIpcMain & {
  emit: (eventName: string, data?: unknown) => void;
  emitTo: (windowId: string, eventName: string, data?: unknown) => void;
};

export const ipcMain: DevIpcMain = {
  handle: frameworkIpcMain.handle.bind(frameworkIpcMain),
  removeHandler: frameworkIpcMain.removeHandler.bind(frameworkIpcMain),
  clearHandlers: frameworkIpcMain.clearHandlers.bind(frameworkIpcMain),
  getHandler: frameworkIpcMain.getHandler.bind(frameworkIpcMain),
  hasHandler: frameworkIpcMain.hasHandler.bind(frameworkIpcMain),
  processRequest: frameworkIpcMain.processRequest.bind(frameworkIpcMain),
  emit(eventName: string, data?: unknown): void {
    emitFrontendEvent(eventName, data);
  },
  emitTo(windowId: string, eventName: string, data?: unknown): void {
    emitFrontendEvent(eventName, data, windowId);
  },
};

