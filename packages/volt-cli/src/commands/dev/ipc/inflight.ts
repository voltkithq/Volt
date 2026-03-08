import type { NativeWindowClosedEvent } from '../types.js';

const ipcInFlightByWindow = new Map<string, number>();

export function tryAcquireWindowIpcSlot(windowId: string, limit: number): boolean {
  const inFlight = ipcInFlightByWindow.get(windowId) ?? 0;
  if (inFlight >= limit) {
    return false;
  }
  ipcInFlightByWindow.set(windowId, inFlight + 1);
  return true;
}

export function releaseWindowIpcSlot(windowId: string): void {
  const inFlight = ipcInFlightByWindow.get(windowId);
  if (inFlight === undefined || inFlight <= 1) {
    ipcInFlightByWindow.delete(windowId);
    return;
  }
  ipcInFlightByWindow.set(windowId, inFlight - 1);
}

function clearIpcLoadStateForWindow(windowId: string): void {
  if (windowId.length === 0) {
    return;
  }
  ipcInFlightByWindow.delete(windowId);
}

export function getWindowIpcInFlightCount(windowId: string): number | undefined {
  return ipcInFlightByWindow.get(windowId);
}

export function handleWindowClosedEventForIpcState(event: NativeWindowClosedEvent): void {
  if (event.jsWindowId === null) {
    clearIpcLoadState();
    return;
  }
  clearIpcLoadStateForWindow(event.windowId);
  if (event.jsWindowId !== event.windowId) {
    clearIpcLoadStateForWindow(event.jsWindowId);
  }
}

export function clearIpcLoadState(): void {
  ipcInFlightByWindow.clear();
}
