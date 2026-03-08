/**
 * Global keyboard shortcut module.
 * Provides registration of global hotkeys that work even when the app is not focused.
 * Requires `permissions: ['globalShortcut']` in volt.config.ts.
 */

import { VoltGlobalShortcut } from '@voltkit/volt-native';

type ShortcutCallback = () => void;

let nativeManager: VoltGlobalShortcut | null = null;
const callbackMap = new Map<string, ShortcutCallback>();

function getNativeManager(): VoltGlobalShortcut {
  if (!nativeManager) {
    nativeManager = new VoltGlobalShortcut();
  }
  return nativeManager;
}

const VALID_MODIFIER_TOKENS = new Set([
  'cmdorctrl',
  'cmd',
  'command',
  'meta',
  'super',
  'ctrl',
  'control',
  'alt',
  'option',
  'shift',
]);

function isValidKeyToken(token: string): boolean {
  const upper = token.toUpperCase();
  if (/^[A-Z]$/.test(upper)) return true;
  if (/^[0-9]$/.test(upper)) return true;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(upper)) return true;
  return ['SPACE', 'ENTER', 'RETURN', 'ESC', 'ESCAPE', 'TAB', 'UP', 'DOWN', 'LEFT', 'RIGHT'].includes(upper);
}

function assertValidAccelerator(accelerator: string): void {
  const parts = accelerator.split('+').map((part) => part.trim()).filter(Boolean);
  if (parts.length < 2) {
    throw new Error(`Invalid accelerator "${accelerator}". Expected at least one modifier and one key.`);
  }

  let keyCount = 0;
  for (const part of parts) {
    const lowered = part.toLowerCase();
    if (VALID_MODIFIER_TOKENS.has(lowered)) {
      continue;
    }
    if (!isValidKeyToken(part)) {
      throw new Error(`Invalid accelerator key token "${part}" in "${accelerator}".`);
    }
    keyCount += 1;
  }

  if (keyCount !== 1) {
    throw new Error(`Invalid accelerator "${accelerator}". Exactly one non-modifier key token is required.`);
  }
}

/**
 * Register a global keyboard shortcut.
 *
 * @example
 * ```ts
 * globalShortcut.register('CmdOrCtrl+Shift+P', () => {
 *   console.log('Shortcut triggered!');
 * });
 * ```
 */
function register(accelerator: string, callback: ShortcutCallback): boolean {
  assertValidAccelerator(accelerator);
  if (callbackMap.has(accelerator)) {
    return false;
  }
  getNativeManager().register(accelerator, () => {
    const cb = callbackMap.get(accelerator);
    if (cb) cb();
  });
  callbackMap.set(accelerator, callback);
  return true;
}

/** Unregister a global keyboard shortcut. */
function unregister(accelerator: string): void {
  assertValidAccelerator(accelerator);
  getNativeManager().unregister(accelerator);
  callbackMap.delete(accelerator);
}

/** Unregister all global keyboard shortcuts. */
function unregisterAll(): void {
  getNativeManager().unregisterAll();
  callbackMap.clear();
}

/** Check if a shortcut is registered. */
function isRegistered(accelerator: string): boolean {
  assertValidAccelerator(accelerator);
  return getNativeManager().isRegistered(accelerator);
}

/** Global shortcut APIs. Requires `permissions: ['globalShortcut']` in volt.config.ts. */
export const globalShortcut = {
  register,
  unregister,
  unregisterAll,
  isRegistered,
};
