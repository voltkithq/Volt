/**
 * Native menu module.
 * Provides Menu and MenuItem classes for building application and context menus.
 * Electron-compatible API surface.
 */

import { VoltMenu } from '@voltkit/volt-native';
import { getApp } from './app.js';

/** Menu item role for predefined system actions. */
export type MenuItemRole =
  | 'quit'
  | 'copy'
  | 'cut'
  | 'paste'
  | 'selectAll'
  | 'undo'
  | 'redo'
  | 'minimize'
  | 'separator';

/** Options for creating a menu item. */
export interface MenuItemOptions {
  /** Internal menu item ID used for native event dispatch. */
  id?: string;
  /** Display label. */
  label?: string;
  /** Keyboard accelerator (e.g., 'CmdOrCtrl+C'). */
  accelerator?: string;
  /** Whether the item is enabled. Default: true. */
  enabled?: boolean;
  /** Item type: 'normal', 'separator', or 'submenu'. */
  type?: 'normal' | 'separator' | 'submenu';
  /** Predefined role. When set, label and accelerator are auto-configured. */
  role?: MenuItemRole;
  /** Click handler. */
  click?: () => void;
  /** Submenu items (only for type 'submenu'). */
  submenu?: MenuItemOptions[];
}

/** A single menu item. */
export class MenuItem {
  readonly id: string | undefined;
  readonly label: string;
  readonly accelerator: string | undefined;
  readonly enabled: boolean;
  readonly type: string;
  readonly role: MenuItemRole | undefined;
  readonly click: (() => void) | undefined;
  readonly submenu: MenuItem[] | undefined;

  constructor(options: MenuItemOptions) {
    const isSeparator = (options.type ?? 'normal') === 'separator';
    const shouldGenerateId = !isSeparator && (!options.role || !!options.click);
    this.id = options.id ?? (shouldGenerateId ? `menu-item-${nextMenuItemId++}` : undefined);
    this.label = options.label ?? '';
    this.accelerator = options.accelerator;
    this.enabled = options.enabled ?? true;
    this.type = options.type ?? 'normal';
    this.role = options.role;
    this.click = options.click;
    if (options.submenu) {
      this.submenu = options.submenu.map((item) => new MenuItem(item));
    }
  }
}

/**
 * Native application menu.
 * Electron-compatible API for building menus.
 *
 * @example
 * ```ts
 * const menu = Menu.buildFromTemplate([
 *   { label: 'File', submenu: [
 *     { label: 'Quit', role: 'quit' },
 *   ]},
 * ]);
 * Menu.setApplicationMenu(menu);
 * ```
 */
export class Menu {
  private items: MenuItem[] = [];
  private _nativeMenu: VoltMenu | null = null;

  private static _applicationMenu: Menu | null = null;

  /** Build a menu from a template array of options. */
  static buildFromTemplate(template: MenuItemOptions[]): Menu {
    ensureMenuPermission('Menu.buildFromTemplate()');
    const menu = new Menu();
    menu.items = template.map((item) => new MenuItem(item));
    syncMenuClickHandlers(menu);

    // Build the native menu from the serializable template
    const nativeTemplate = menu.toJSON();
    menu._nativeMenu = new VoltMenu(nativeTemplate);

    return menu;
  }

  /** Set the application menu bar. */
  static setApplicationMenu(menu: Menu | null): void {
    ensureMenuPermission('Menu.setApplicationMenu()');
    const previousMenu = Menu._applicationMenu;
    if (menu) {
      const nextNativeMenu = menu._nativeMenu ?? new VoltMenu(menu.toJSON());
      syncMenuClickHandlers(menu);
      try {
        nextNativeMenu.setAsAppMenu();
      } catch (err) {
        if (previousMenu !== menu) {
          clearMenuClickHandlers(menu);
        }
        throw err;
      }
      if (previousMenu && previousMenu !== menu) {
        clearMenuClickHandlers(previousMenu);
      }
      menu._nativeMenu = nextNativeMenu;
      Menu._applicationMenu = menu;
      return;
    }
    // Explicitly clear native app menu when caller passes null.
    new VoltMenu([]).setAsAppMenu();
    if (previousMenu) {
      clearMenuClickHandlers(previousMenu);
    }
    Menu._applicationMenu = null;
  }

  /** Get the current application menu. */
  static getApplicationMenu(): Menu | null {
    return Menu._applicationMenu;
  }

  /** Append a menu item. */
  append(item: MenuItem): void {
    ensureMenuPermission('Menu.append()');
    this.items.push(item);
    this._nativeMenu = new VoltMenu(this.toJSON());
    syncMenuClickHandlers(this);
    if (Menu._applicationMenu === this) {
      this._nativeMenu.setAsAppMenu();
    }
  }

  /** Get all items in this menu. */
  getItems(): MenuItem[] {
    return [...this.items];
  }

  /** Convert menu to a serializable template for passing to native code. */
  toJSON(): MenuItemOptions[] {
    return this.items.map((item) => serializeMenuItem(item));
  }
}

/** Recursively serialize a MenuItem to a plain object for native. */
function serializeMenuItem(item: MenuItem): MenuItemOptions {
  const result: MenuItemOptions = {
    id: item.id,
    label: item.label,
    accelerator: item.accelerator,
    enabled: item.enabled,
    type: item.type as MenuItemOptions['type'],
    role: item.role,
  };
  if (item.submenu) {
    result.submenu = item.submenu.map((sub) => serializeMenuItem(sub));
  }
  return result;
}

const menuClickHandlers = new Map<string, () => void>();
const menuHandlerIds = new WeakMap<Menu, Set<string>>();
let nextMenuItemId = 1;

function ensureMenuPermission(apiName: string): void {
  const granted = new Set(getApp().getConfig().permissions ?? []);
  if (!granted.has('menu')) {
    throw new Error(
      `Permission denied: ${apiName} requires 'menu' in volt.config.ts permissions.`,
    );
  }
}

function syncMenuClickHandlers(menu: Menu): void {
  clearMenuClickHandlers(menu);
  const ids = new Set<string>();
  collectMenuClickHandlers(menu.getItems(), ids);
  menuHandlerIds.set(menu, ids);
}

function clearMenuClickHandlers(menu: Menu): void {
  const ids = menuHandlerIds.get(menu);
  if (!ids) {
    return;
  }
  for (const id of ids) {
    menuClickHandlers.delete(id);
  }
  menuHandlerIds.delete(menu);
}

function collectMenuClickHandlers(items: MenuItem[], ids: Set<string>): void {
  for (const item of items) {
    if (item.id && item.click) {
      menuClickHandlers.set(item.id, item.click);
      ids.add(item.id);
    }
    if (item.submenu) {
      collectMenuClickHandlers(item.submenu, ids);
    }
  }
}

/** @internal Dispatches a native menu event into the registered JS click callback. */
export function __internalDispatchMenuEvent(menuId: string): void {
  const clickHandler = menuClickHandlers.get(menuId);
  if (!clickHandler) {
    return;
  }
  try {
    clickHandler();
  } catch (err) {
    console.error('[volt] Menu click handler failed:', err);
  }
}
