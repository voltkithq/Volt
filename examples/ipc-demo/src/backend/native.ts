import { ipcMain } from 'volt:ipc';
import * as voltGlobalShortcut from 'volt:globalShortcut';
import * as voltMenu from 'volt:menu';
import * as voltTray from 'volt:tray';
import * as voltWindow from 'volt:window';

import { SHORTCUT_ACCELERATOR, runtimeState, toErrorMessage } from './state.js';

export async function ensureNativeIntegrations(): Promise<{
  shortcutRegistered: boolean;
  trayReady: boolean;
}> {
  if (!runtimeState.menuConfigured) {
    await voltMenu.setAppMenu([
      {
        label: 'Demo',
        type: 'submenu',
        submenu: [
          { id: 'demo:refresh-status', label: 'Refresh Status', accelerator: 'CmdOrCtrl+R' },
          { type: 'separator' },
          { id: 'demo:quit', label: 'Quit', accelerator: 'CmdOrCtrl+Q' },
        ],
      },
    ]);
    runtimeState.menuConfigured = true;
  }

  if (!runtimeState.shortcutRegistered) {
    try {
      await voltGlobalShortcut.register(SHORTCUT_ACCELERATOR);
      runtimeState.shortcutRegistered = true;
    } catch (error) {
      const message = toErrorMessage(error).toLowerCase();
      if (message.includes('already')) {
        runtimeState.shortcutRegistered = true;
      } else {
        throw error;
      }
    }
  }

  if (!runtimeState.trayReady) {
    try {
      await voltTray.create({ tooltip: 'Volt IPC Demo' });
      runtimeState.trayReady = true;
    } catch (error) {
      ipcMain.emit('demo:native-error', {
        feature: 'tray',
        message: toErrorMessage(error),
      });
    }
  }

  return {
    shortcutRegistered: runtimeState.shortcutRegistered,
    trayReady: runtimeState.trayReady,
  };
}

export function registerNativeEventBridge(): void {
  voltMenu.on('click', (payload: unknown) => {
    const menuId = (payload as { menuId?: unknown })?.menuId;
    if (menuId === 'demo:quit') {
      voltWindow.quit();
      return;
    }
    ipcMain.emit('demo:menu-click', payload ?? null);
  });

  voltGlobalShortcut.on('triggered', (payload: unknown) => {
    ipcMain.emit('demo:shortcut', {
      accelerator: SHORTCUT_ACCELERATOR,
      ...(payload && typeof payload === 'object' ? payload : {}),
    });
  });

  voltTray.on('click', (payload: unknown) => {
    ipcMain.emit('demo:tray-click', payload ?? null);
  });
}
