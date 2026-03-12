import { ipcMain } from 'volt:ipc';
import {
  buildSyncStormPreset,
  startSyncStorm,
  type SyncStormSummary,
} from './backend-logic.js';

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
    tick() {
      return;
    },
    snapshot() {
      return;
    },
    complete(result) {
      activeScenarioId = null;
      lastSummary = result;
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
