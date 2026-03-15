import * as voltDb from 'volt:db';
import * as voltSecureStorage from 'volt:secureStorage';

import { ensureDatabase, insertDbRecord, listDbRecords } from './database.js';
import { ensureNativeIntegrations } from './native.js';
import {
  parseSecretKeyPayload,
  parseSecretSetPayload,
  SHORTCUT_ACCELERATOR,
  sleep,
} from './state.js';
import { buildStatusPayload } from './status.js';

interface DemoHandlersContext {
  ipcMain: typeof import('volt:ipc').ipcMain;
  voltEvents: typeof import('volt:events');
  voltWindow: typeof import('volt:window');
}

export function registerDemoHandlers({
  ipcMain,
  voltEvents,
  voltWindow,
}: DemoHandlersContext): void {
  ipcMain.handle('native:setup', async () => {
    const result = await ensureNativeIntegrations();
    ipcMain.emit('demo:native-ready', {
      shortcut: SHORTCUT_ACCELERATOR,
      ...result,
    });
    return {
      ...result,
      shortcut: SHORTCUT_ACCELERATOR,
    };
  });

  ipcMain.handle('window:minimize', () => {
    voltWindow.minimize();
    return { ok: true };
  });

  ipcMain.handle('window:maximize', () => {
    voltWindow.maximize();
    return { ok: true };
  });

  ipcMain.handle('window:restore', () => {
    voltWindow.restore();
    return { ok: true };
  });

  ipcMain.handle('window:count', async () => {
    const count = await voltWindow.getWindowCount();
    return { count };
  });

  ipcMain.handle('progress:run', async () => {
    for (const percent of [0, 20, 40, 60, 80, 100]) {
      ipcMain.emit('demo:progress', { percent, at: Date.now() });
      if (percent < 100) {
        await sleep(180);
      }
    }
    return { done: true };
  });

  ipcMain.handle('db:add', async (data: unknown) => {
    const message = (data as { message?: unknown })?.message;
    if (typeof message !== 'string') {
      throw new Error('db:add.message must be a string');
    }

    const record = await insertDbRecord(message);
    ipcMain.emit('demo:db-updated', { action: 'add', id: record.id });
    return record;
  });

  ipcMain.handle('db:list', async () => listDbRecords());

  ipcMain.handle('db:clear', async () => {
    await ensureDatabase();
    const result = await voltDb.execute('DELETE FROM demo_records');
    ipcMain.emit('demo:db-updated', { action: 'clear' });
    return result;
  });

  ipcMain.handle('secure-storage:set', async (data: unknown) => {
    const { key, value } = parseSecretSetPayload(data);
    await voltSecureStorage.set(key, value);
    const has = await voltSecureStorage.has(key);
    ipcMain.emit('demo:secure-storage-updated', { action: 'set', key, has });
    return { ok: true, key, has };
  });

  ipcMain.handle('secure-storage:get', async (data: unknown) => {
    const key = parseSecretKeyPayload(data);
    const value = await voltSecureStorage.get(key);
    return { key, value, has: value !== null };
  });

  ipcMain.handle('secure-storage:has', async (data: unknown) => {
    const key = parseSecretKeyPayload(data);
    const has = await voltSecureStorage.has(key);
    return { key, has };
  });

  ipcMain.handle('secure-storage:delete', async (data: unknown) => {
    const key = parseSecretKeyPayload(data);
    await voltSecureStorage.delete(key);
    const has = await voltSecureStorage.has(key);
    ipcMain.emit('demo:secure-storage-updated', { action: 'delete', key, has });
    return { ok: true, key, has };
  });

  ipcMain.handle('status', async () => buildStatusPayload(voltEvents, voltWindow));
}
