import { ipcMain } from 'volt:ipc';
import { getAnalyticsProfile, runAnalyticsBenchmark } from './backend-logic.js';

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
