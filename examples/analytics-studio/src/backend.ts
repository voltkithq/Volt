import { ipcMain } from 'volt:ipc';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';
import { getAnalyticsProfile, runAnalyticsBenchmark } from './backend-logic.js';

const RESULT_FILE = '.volt-benchmark-result.json';

ipcMain.handle('analytics:profile', (payload: unknown) => {
  const datasetSize = Number((payload as { datasetSize?: unknown } | null)?.datasetSize ?? 24_000);
  return getAnalyticsProfile(Number.isFinite(datasetSize) ? datasetSize : 24_000);
});

ipcMain.handle('analytics:run', (payload: unknown) => {
  const request = (payload as Record<string, unknown> | null) ?? {};
  return runAnalyticsBenchmark({
    datasetSize: request.datasetSize,
    iterations: request.iterations,
    searchTerm: request.searchTerm,
    minScore: request.minScore,
    topN: request.topN,
  });
});

ipcMain.handle('benchmark:complete', async (payload: unknown) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
