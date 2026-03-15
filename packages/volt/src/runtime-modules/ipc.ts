declare module 'volt:ipc' {
  export interface IpcMain {
    handle(channel: string, handler: (args: unknown) => unknown | Promise<unknown>): void;
    removeHandler(channel: string): void;
    clearHandlers(): void;
    hasHandler(channel: string): boolean;
    emit(eventName: string, data?: unknown): void;
    emitTo(windowId: string, eventName: string, data?: unknown): void;
  }

  export const ipcMain: IpcMain;
}

declare module 'volt:events' {
  export function emit(eventName: string, data?: unknown): void;
  export function emitTo(windowId: string, eventName: string, data?: unknown): void;
}

declare module 'volt:window' {
  export function close(windowId?: string): void;
  export function show(windowId?: string): void;
  export function focus(windowId?: string): void;
  export function maximize(windowId?: string): void;
  export function minimize(windowId?: string): void;
  export function restore(windowId?: string): void;
  export function getWindowCount(): Promise<number>;
  export function quit(): void;
}
