import type { BrowserWindowRegistryEntry } from './types.js';

const windows = new Map<string, BrowserWindowRegistryEntry>();
let focusedWindow: BrowserWindowRegistryEntry | null = null;

export function addWindowToRegistry(window: BrowserWindowRegistryEntry): void {
  windows.set(window.getId(), window);
}

export function removeWindowFromRegistry(window: BrowserWindowRegistryEntry): boolean {
  windows.delete(window.getId());
  if (focusedWindow === window) {
    focusedWindow = null;
  }
  return windows.size === 0;
}

export function focusWindowInRegistry(window: BrowserWindowRegistryEntry): void {
  if (!window.isDestroyed()) {
    focusedWindow = window;
  }
}

export function blurWindowInRegistry(window: BrowserWindowRegistryEntry): void {
  if (focusedWindow === window) {
    focusedWindow = null;
  }
}

export function getRegisteredWindows<T extends BrowserWindowRegistryEntry>(): T[] {
  return Array.from(windows.values()) as T[];
}

export function getFocusedRegisteredWindow<T extends BrowserWindowRegistryEntry>(): T | null {
  return focusedWindow as T | null;
}

export function getRegisteredWindowById<T extends BrowserWindowRegistryEntry>(id: string): T | undefined {
  return windows.get(id) as T | undefined;
}
