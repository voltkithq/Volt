import * as voltClipboard from 'volt:clipboard';
import * as voltDb from 'volt:db';
import * as voltOs from 'volt:os';
import * as voltSecureStorage from 'volt:secureStorage';

import {
  evaluateNativeReady,
  extractDbRowsCount,
  summarizeClipboardRead,
} from '../backend-logic.js';

import { ensureDatabase } from './database.js';
import { buildUuidLike, DEFAULT_SECRET_KEY, runtimeState, SHORTCUT_ACCELERATOR } from './state.js';

export async function buildStatusPayload(
  voltEvents: typeof import('volt:events'),
  voltWindow: typeof import('volt:window'),
) {
  await ensureDatabase();
  voltEvents.emit('demo:events-module', { at: Date.now() });

  let clipboardValue: unknown;
  try {
    clipboardValue = voltClipboard.readText();
  } catch {
    clipboardValue = '';
  }

  const clipboard = summarizeClipboardRead(clipboardValue);
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
        menuConfigured: runtimeState.menuConfigured,
        shortcutRegistered: runtimeState.shortcutRegistered,
        trayReady: runtimeState.trayReady,
      }),
      shortcut: SHORTCUT_ACCELERATOR,
      secureStorageDemoKey: DEFAULT_SECRET_KEY,
      secureStorageHasDemoKey,
    },
  };
}
