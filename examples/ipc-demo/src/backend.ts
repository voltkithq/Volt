import { ipcMain } from 'volt:ipc';
import { registerContractHandlers } from 'voltkit/ipc-contract';
import * as voltClipboard from 'volt:clipboard';
import * as voltCrypto from 'volt:crypto';
import * as voltDb from 'volt:db';
import * as voltEvents from 'volt:events';
import * as voltGlobalShortcut from 'volt:globalShortcut';
import * as voltMenu from 'volt:menu';
import * as voltOs from 'volt:os';
import * as voltSecureStorage from 'volt:secureStorage';
import * as voltTray from 'volt:tray';
import * as voltWindow from 'volt:window';
import {
  type DbRecord,
  evaluateNativeReady,
  extractDbRowsCount,
  formatUuidFromHash,
  normalizeSecretKey,
  normalizeSecretValue,
  summarizeClipboardRead,
  toDbRecords,
} from './backend-logic.js';
import {
  ipcCommands,
  type ComputeRequest,
  type EchoRequest,
  type PingResult,
} from './ipc-contract.js';

const SHORTCUT_ACCELERATOR = 'CmdOrCtrl+Shift+P';
const DB_PATH = 'ipc-demo/records.sqlite';
const DEFAULT_SECRET_KEY = 'ipc-demo/demo-secret';
let databaseReady = false;
let menuConfigured = false;
let shortcutRegistered = false;
let trayReady = false;

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function buildUuidLike(seed: string): string {
  return formatUuidFromHash(voltCrypto.sha256(seed));
}

function buildRecordId(seed: string): string {
  return buildUuidLike(`record:${seed}:${Math.random()}`);
}

async function sleep(milliseconds: number): Promise<void> {
  await new Promise<void>((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

async function ensureDatabase(): Promise<void> {
  if (databaseReady) {
    return;
  }

  await voltDb.open(DB_PATH);
  await voltDb.execute(
    `CREATE TABLE IF NOT EXISTS demo_records (
      id TEXT PRIMARY KEY,
      message TEXT NOT NULL,
      created_at INTEGER NOT NULL
    )`,
  );
  databaseReady = true;
}

async function insertDbRecord(message: string): Promise<DbRecord> {
  await ensureDatabase();
  const trimmed = message.trim();
  if (!trimmed) {
    throw new Error('db message must not be empty');
  }

  const record: DbRecord = {
    id: buildRecordId(trimmed),
    message: trimmed,
    createdAt: Date.now(),
  };
  await voltDb.execute(
    'INSERT INTO demo_records (id, message, created_at) VALUES (?, ?, ?)',
    [record.id, record.message, record.createdAt],
  );
  return record;
}

async function listDbRecords(): Promise<DbRecord[]> {
  await ensureDatabase();
  const rows = await voltDb.query(
    'SELECT id, message, created_at FROM demo_records ORDER BY created_at DESC LIMIT 12',
  );
  return toDbRecords(rows);
}

function parseSecretKeyPayload(data: unknown): string {
  const key = (data as { key?: unknown } | null)?.key;
  return normalizeSecretKey(key);
}

function parseSecretSetPayload(data: unknown): { key: string; value: string } {
  const payload = (data as { key?: unknown; value?: unknown } | null) ?? {};
  return {
    key: normalizeSecretKey(payload.key),
    value: normalizeSecretValue(payload.value),
  };
}

async function ensureNativeIntegrations(): Promise<{ shortcutRegistered: boolean; trayReady: boolean }> {
  if (!menuConfigured) {
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
    menuConfigured = true;
  }

  if (!shortcutRegistered) {
    try {
      await voltGlobalShortcut.register(SHORTCUT_ACCELERATOR);
      shortcutRegistered = true;
    } catch (error) {
      const message = toErrorMessage(error).toLowerCase();
      if (message.includes('already')) {
        shortcutRegistered = true;
      } else {
        throw error;
      }
    }
  }

  if (!trayReady) {
    try {
      await voltTray.create({ tooltip: 'Volt IPC Demo' });
      trayReady = true;
    } catch (error) {
      ipcMain.emit('demo:native-error', {
        feature: 'tray',
        message: toErrorMessage(error),
      });
    }
  }

  return { shortcutRegistered, trayReady };
}

function registerNativeEventBridge(): void {
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

registerNativeEventBridge();

registerContractHandlers(ipcMain, ipcCommands, {
  'demo.ping': (_payload): PingResult => ({ pong: Date.now() }),
  'demo.echo': (payload: EchoRequest) => payload,
  'demo.compute': (payload: ComputeRequest) => ({
    sum: payload.a + payload.b,
    product: payload.a * payload.b,
  }),
});

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

ipcMain.handle('status', async () => {
  await ensureDatabase();
  voltEvents.emit('demo:events-module', { at: Date.now() });

  const clipboard = summarizeClipboardRead(voltClipboard.readText());
  const dbRowsRaw = await voltDb.queryOne('SELECT COUNT(*) AS total FROM demo_records');
  const windowCount = await voltWindow.getWindowCount();
  const secureStorageHasDemoKey = await voltSecureStorage.has(DEFAULT_SECRET_KEY);

  return {
    os: {
      platform: voltOs.platform(),
      arch: voltOs.arch(),
    },
    generatedUuid: buildUuidLike(`status:${Date.now()}:${Math.random()}`),
    clipboard,
    runtime: {
      windowCount,
      dbRows: extractDbRowsCount(dbRowsRaw),
      nativeReady: evaluateNativeReady({
        menuConfigured,
        shortcutRegistered,
        trayReady,
      }),
      shortcut: SHORTCUT_ACCELERATOR,
      secureStorageDemoKey: DEFAULT_SECRET_KEY,
      secureStorageHasDemoKey,
    },
  };
});
