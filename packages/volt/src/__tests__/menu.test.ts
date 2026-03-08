import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { VoltMenu } from '@voltkit/volt-native';
import { createApp, resetApp } from '../app.js';
import { __internalDispatchMenuEvent, Menu, MenuItem } from '../menu.js';

type VoltMenuTestState = {
  resetInstances: () => void;
  instances: Array<{ itemCount: () => number; setAsAppMenu: ReturnType<typeof vi.fn> }>;
};

function voltMenuState(): VoltMenuTestState {
  return VoltMenu as unknown as VoltMenuTestState;
}

describe('MenuItem', () => {
  it('applies default values', () => {
    const item = new MenuItem({});
    expect(item.label).toBe('');
    expect(item.enabled).toBe(true);
    expect(item.type).toBe('normal');
    expect(item.accelerator).toBeUndefined();
    expect(item.role).toBeUndefined();
    expect(item.click).toBeUndefined();
    expect(item.submenu).toBeUndefined();
  });

  it('accepts all options', () => {
    const clickFn = () => {};
    const item = new MenuItem({
      label: 'Edit',
      accelerator: 'CmdOrCtrl+E',
      enabled: false,
      type: 'normal',
      role: 'copy',
      click: clickFn,
    });
    expect(item.label).toBe('Edit');
    expect(item.accelerator).toBe('CmdOrCtrl+E');
    expect(item.enabled).toBe(false);
    expect(item.role).toBe('copy');
    expect(item.click).toBe(clickFn);
  });

  it('recursively creates submenu items', () => {
    const item = new MenuItem({
      label: 'File',
      type: 'submenu',
      submenu: [
        { label: 'New', accelerator: 'CmdOrCtrl+N' },
        { label: 'Open', accelerator: 'CmdOrCtrl+O' },
        { type: 'separator' },
        { label: 'Quit', role: 'quit' },
      ],
    });
    expect(item.submenu).toBeDefined();
    expect(item.submenu).toHaveLength(4);
    expect(item.submenu![0]).toBeInstanceOf(MenuItem);
    expect(item.submenu![0].label).toBe('New');
    expect(item.submenu![2].type).toBe('separator');
    expect(item.submenu![3].role).toBe('quit');
  });

  it('assigns an id when a role-based item also has a click handler', () => {
    const item = new MenuItem({
      role: 'copy',
      click: () => {},
    });
    expect(item.id).toBeDefined();
  });
});

describe('Menu', () => {
  beforeEach(() => {
    resetApp();
    createApp({
      name: 'Menu Test App',
      permissions: ['menu'],
    });
    Menu.setApplicationMenu(null);
    voltMenuState().resetInstances();
  });

  afterEach(() => {
    resetApp();
  });

  it('buildFromTemplate creates a Menu with items', () => {
    const menu = Menu.buildFromTemplate([
      { label: 'File', type: 'submenu', submenu: [{ label: 'Quit', role: 'quit' }] },
      { label: 'Edit', type: 'submenu', submenu: [{ label: 'Copy', role: 'copy' }] },
    ]);
    const items = menu.getItems();
    expect(items).toHaveLength(2);
    expect(items[0].label).toBe('File');
    expect(items[1].label).toBe('Edit');
  });

  it('append adds items', () => {
    const menu = Menu.buildFromTemplate([]);
    menu.append(new MenuItem({ label: 'Added' }));
    expect(menu.getItems()).toHaveLength(1);
    expect(menu.getItems()[0].label).toBe('Added');
  });

  it('getItems returns a copy (not the internal array)', () => {
    const menu = Menu.buildFromTemplate([{ label: 'A' }]);
    const items1 = menu.getItems();
    const items2 = menu.getItems();
    expect(items1).not.toBe(items2);
    expect(items1).toEqual(items2);
  });

  it('toJSON produces serializable template', () => {
    const menu = Menu.buildFromTemplate([
      {
        label: 'File',
        type: 'submenu',
        submenu: [
          { label: 'New', accelerator: 'CmdOrCtrl+N' },
        ],
      },
    ]);
    const json = menu.toJSON();
    expect(json).toHaveLength(1);
    expect(json[0].label).toBe('File');
    expect(json[0].submenu).toHaveLength(1);
    expect(json[0].submenu![0].label).toBe('New');
    // click should not be serialized (function)
    expect(json[0].click).toBeUndefined();
  });

  it('setApplicationMenu / getApplicationMenu work', () => {
    expect(Menu.getApplicationMenu()).toBeNull();
    const menu = Menu.buildFromTemplate([{ label: 'App' }]);
    Menu.setApplicationMenu(menu);
    expect(Menu.getApplicationMenu()).toBe(menu);
  });

  it('setApplicationMenu materializes native menu even when created via constructor', () => {
    const menu = new Menu();
    menu.append(new MenuItem({ label: 'App' }));
    Menu.setApplicationMenu(menu);
    const nativeMenu = (menu as unknown as { _nativeMenu: { setAsAppMenu: () => void } | null })._nativeMenu;
    expect(nativeMenu).not.toBeNull();
  });

  it('append re-syncs native app menu when menu is active', () => {
    const menu = Menu.buildFromTemplate([{ label: 'Initial' }]);
    Menu.setApplicationMenu(menu);
    const nativeBefore = (menu as unknown as { _nativeMenu: any })._nativeMenu;
    expect(nativeBefore.setAsAppMenu).toHaveBeenCalledTimes(1);

    menu.append(new MenuItem({ label: 'Added' }));

    const nativeAfter = (menu as unknown as { _nativeMenu: any })._nativeMenu;
    expect(nativeAfter).not.toBe(nativeBefore);
    expect(nativeAfter.setAsAppMenu).toHaveBeenCalledTimes(1);
  });

  it('setApplicationMenu(null) clears the menu', () => {
    const menu = Menu.buildFromTemplate([{ label: 'App' }]);
    Menu.setApplicationMenu(menu);
    Menu.setApplicationMenu(null);
    expect(Menu.getApplicationMenu()).toBeNull();
  });

  it('does not mutate application menu state when native set fails', () => {
    const current = Menu.buildFromTemplate([{ label: 'Current' }]);
    Menu.setApplicationMenu(current);

    const next = Menu.buildFromTemplate([{ label: 'Next' }]);
    const nativeNext = (next as unknown as { _nativeMenu: { setAsAppMenu: ReturnType<typeof vi.fn> } })._nativeMenu;
    nativeNext.setAsAppMenu.mockImplementationOnce(() => {
      throw new Error('native set failed');
    });

    expect(() => Menu.setApplicationMenu(next)).toThrow('native set failed');
    expect(Menu.getApplicationMenu()).toBe(current);
  });

  it('setApplicationMenu(null) clears native app menu state', () => {
    const menu = Menu.buildFromTemplate([{ label: 'App' }]);
    Menu.setApplicationMenu(menu);
    voltMenuState().resetInstances();

    Menu.setApplicationMenu(null);

    expect(voltMenuState().instances).toHaveLength(1);
    const clearMenu = voltMenuState().instances[0];
    expect(clearMenu.itemCount()).toBe(0);
    expect(clearMenu.setAsAppMenu).toHaveBeenCalledTimes(1);
  });

  it('requires the menu permission before building native menus', () => {
    resetApp();
    createApp({ name: 'Menu Test App' });

    expect(() => Menu.buildFromTemplate([{ label: 'App' }])).toThrow(
      "requires 'menu' in volt.config.ts permissions",
    );
  });

  it('dispatches registered click callback for native menu events', () => {
    let clicked = false;
    const menu = Menu.buildFromTemplate([
      {
        label: 'File',
        submenu: [
          {
            label: 'Open',
            click: () => {
              clicked = true;
            },
          },
        ],
      },
    ]);
    Menu.setApplicationMenu(menu);
    const submenuId = menu.toJSON()[0].submenu?.[0].id as string;

    __internalDispatchMenuEvent(submenuId);

    expect(clicked).toBe(true);
  });

  it('wires click callback immediately when built from template', () => {
    let clicked = false;
    const menu = Menu.buildFromTemplate([
      {
        label: 'Quick',
        click: () => {
          clicked = true;
        },
      },
    ]);
    const id = menu.toJSON()[0].id as string;

    __internalDispatchMenuEvent(id);

    expect(clicked).toBe(true);
  });

  it('keeps application menu callbacks active after other menus are built', () => {
    let appClicked = false;
    const appMenu = Menu.buildFromTemplate([
      {
        label: 'File',
        submenu: [{ label: 'Open', click: () => { appClicked = true; } }],
      },
    ]);
    Menu.setApplicationMenu(appMenu);
    const appItemId = appMenu.toJSON()[0].submenu?.[0].id as string;

    // Build another menu after app menu registration.
    Menu.buildFromTemplate([{ label: 'Ctx', click: () => {} }]);

    __internalDispatchMenuEvent(appItemId);
    expect(appClicked).toBe(true);
  });

  it('append updates callbacks for non-application menus', () => {
    let clicked = false;
    const contextMenu = Menu.buildFromTemplate([{ label: 'Base' }]);
    const item = new MenuItem({ label: 'Dynamic', click: () => { clicked = true; } });

    contextMenu.append(item);
    __internalDispatchMenuEvent(item.id as string);

    expect(clicked).toBe(true);
  });
});
