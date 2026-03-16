import { globalShortcut } from 'voltkit';
import { devModuleError } from './shared.js';

type ShortcutHandler = (payload: unknown) => void;

const registrations = new Map<string, number>();
const listeners = new Set<ShortcutHandler>();
let nextShortcutId = 1;

function emitTriggered(payload: unknown): void {
  for (const listener of listeners) {
    try {
      listener(payload);
    } catch {
      // Preserve handler isolation in backend event fan-out.
    }
  }
}

export async function register(accelerator: string): Promise<number> {
  const normalizedAccelerator = accelerator.trim();
  if (!normalizedAccelerator) {
    throw devModuleError('globalShortcut', 'Accelerator must be a non-empty string.');
  }

  const existing = registrations.get(normalizedAccelerator);
  if (existing !== undefined) {
    return existing;
  }

  const id = nextShortcutId;
  nextShortcutId += 1;

  const registered = globalShortcut.register(normalizedAccelerator, () => {
    emitTriggered({ id, accelerator: normalizedAccelerator });
  });
  if (!registered) {
    throw devModuleError(
      'globalShortcut',
      `Global shortcut already registered: ${normalizedAccelerator}`,
    );
  }

  registrations.set(normalizedAccelerator, id);
  return id;
}

export async function unregister(accelerator: string): Promise<void> {
  const normalizedAccelerator = accelerator.trim();
  if (!normalizedAccelerator) {
    return;
  }
  globalShortcut.unregister(normalizedAccelerator);
  registrations.delete(normalizedAccelerator);
}

export async function unregisterAll(): Promise<void> {
  globalShortcut.unregisterAll();
  registrations.clear();
}

export function on(eventName: 'triggered', handler: (payload: unknown) => void): void {
  if (eventName !== 'triggered') {
    throw devModuleError('globalShortcut', `Unsupported shortcut event "${eventName}".`);
  }
  listeners.add(handler);
}

export function off(eventName: 'triggered', handler: (payload: unknown) => void): void {
  if (eventName !== 'triggered') {
    return;
  }
  listeners.delete(handler);
}
