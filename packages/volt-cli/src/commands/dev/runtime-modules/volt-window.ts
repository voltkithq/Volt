import { BrowserWindow, getApp } from 'voltkit';

function resolveTargetWindow(windowId?: string): BrowserWindow | undefined {
  if (typeof windowId === 'string' && windowId.trim().length > 0) {
    return BrowserWindow.fromId(windowId.trim());
  }
  return BrowserWindow.getFocusedWindow() ?? BrowserWindow.getAllWindows()[0];
}

function withTargetWindow(windowId: string | undefined, action: (window: BrowserWindow) => void): void {
  const target = resolveTargetWindow(windowId);
  if (!target) {
    return;
  }
  action(target);
}

export function close(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.close();
  });
}

export function show(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.show();
  });
}

export function focus(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.focus();
  });
}

export function maximize(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.maximize();
  });
}

export function minimize(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.minimize();
  });
}

export function restore(windowId?: string): void {
  withTargetWindow(windowId, (window) => {
    window.restore();
  });
}

export async function getWindowCount(): Promise<number> {
  return BrowserWindow.getAllWindows().length;
}

export function quit(): void {
  for (const window of BrowserWindow.getAllWindows()) {
    window.close();
  }
  try {
    getApp().quit();
  } catch {
    // Ignore if app is not initialized in isolated runtime tests.
  }
}

