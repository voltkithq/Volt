import { ipcMain } from 'volt:ipc';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';
import {
  buildSyncStormPreset,
  startSyncStorm,
  type SyncStormSummary,
} from './backend-logic.js';

const RESULT_FILE = '.volt-benchmark-result.json';
let activeScenarioId: string | null = null;
let lastSummary: SyncStormSummary | null = null;

ipcMain.handle('sync:status', () => ({
  activeScenarioId,
  lastSummary,
  preset: buildSyncStormPreset(),
}));

ipcMain.handle('sync:run', (payload: unknown) => {
  if (activeScenarioId) {
    throw new Error(`sync scenario already running: ${activeScenarioId}`);
  }

  const execution = startSyncStorm((payload as Record<string, unknown> | null) ?? {}, {
    tick(result) {
      ipcMain.emit('sync:tick', result);
    },
    snapshot(result) {
      ipcMain.emit('sync:snapshot', result);
    },
    complete(result) {
      activeScenarioId = null;
      lastSummary = result;
      ipcMain.emit('sync:complete', result);
    },
  });

  activeScenarioId = execution.scenarioId;
  void execution.done.catch(() => {
    activeScenarioId = null;
  });

  return {
    scenarioId: execution.scenarioId,
    config: execution.config,
  };
});

ipcMain.handle('benchmark:complete', async (payload: unknown) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
