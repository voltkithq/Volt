import { Tray } from 'voltkit';
import { devModuleError } from './shared.js';

type TrayClickHandler = (payload: unknown) => void;

const clickHandlers = new Set<TrayClickHandler>();
let tray: Tray | null = null;

function emitTrayClick(payload: unknown): void {
  for (const handler of clickHandlers) {
    try {
      handler(payload);
    } catch {
      // Preserve handler isolation in backend event fan-out.
    }
  }
}

function currentTray(): Tray {
  if (!tray) {
    throw devModuleError('tray', 'Tray is not created. Call create() first.');
  }
  return tray;
}

export async function create(options: unknown = {}): Promise<void> {
  if (tray) {
    return;
  }

  const raw = (options && typeof options === 'object') ? options as Record<string, unknown> : {};
  tray = new Tray({
    tooltip: typeof raw.tooltip === 'string' ? raw.tooltip : undefined,
    icon: typeof raw.icon === 'string' ? raw.icon : undefined,
  });
  tray.on('click', (payload: unknown) => {
    emitTrayClick(payload ?? null);
  });
}

export function setTooltip(tooltip: string): void {
  currentTray().setToolTip(tooltip);
}

export function setVisible(visible: boolean): void {
  currentTray().setVisible(visible);
}

export function destroy(): void {
  if (!tray) {
    return;
  }
  tray.destroy();
  tray = null;
}

export function on(eventName: 'click', handler: (payload: unknown) => void): void {
  if (eventName !== 'click') {
    throw devModuleError('tray', `Unsupported tray event "${eventName}".`);
  }
  clickHandlers.add(handler);
}

export function off(eventName: 'click', handler: (payload: unknown) => void): void {
  if (eventName !== 'click') {
    return;
  }
  clickHandlers.delete(handler);
}
