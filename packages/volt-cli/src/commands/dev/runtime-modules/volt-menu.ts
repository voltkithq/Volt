import { Menu, type MenuItemOptions } from 'voltkit';
import { devModuleError } from './shared.js';

type MenuClickHandler = (payload: unknown) => void;

const clickHandlers = new Set<MenuClickHandler>();

function emitMenuClick(payload: unknown): void {
  for (const handler of clickHandlers) {
    try {
      handler(payload);
    } catch {
      // Preserve handler isolation in backend event fan-out.
    }
  }
}

function decorateMenuItem(raw: unknown): MenuItemOptions {
  const value = (raw && typeof raw === 'object') ? raw as Record<string, unknown> : {};
  const id = typeof value.id === 'string' ? value.id : undefined;
  const originalClick = typeof value.click === 'function'
    ? value.click as () => void
    : undefined;
  const submenuRaw = Array.isArray(value.submenu) ? value.submenu : undefined;
  const submenu = submenuRaw?.map((entry) => decorateMenuItem(entry));

  const item: MenuItemOptions = {
    ...value as MenuItemOptions,
    id,
    submenu,
  };

  if (id) {
    item.click = () => {
      try {
        originalClick?.();
      } finally {
        emitMenuClick({ menuId: id });
      }
    };
  } else if (originalClick) {
    item.click = originalClick;
  }

  return item;
}

function normalizeTemplate(template: unknown): MenuItemOptions[] {
  if (!Array.isArray(template)) {
    throw devModuleError('menu', 'Menu template must be an array.');
  }
  return template.map((item) => decorateMenuItem(item));
}

export async function setAppMenu(template: unknown): Promise<void> {
  const menu = Menu.buildFromTemplate(normalizeTemplate(template));
  Menu.setApplicationMenu(menu);
}

export function on(eventName: 'click', handler: (payload: unknown) => void): void {
  if (eventName !== 'click') {
    throw devModuleError('menu', `Unsupported menu event "${eventName}".`);
  }
  clickHandlers.add(handler);
}

export function off(eventName: 'click', handler: (payload: unknown) => void): void {
  if (eventName !== 'click') {
    return;
  }
  clickHandlers.delete(handler);
}
