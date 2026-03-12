import { ipcMain } from 'volt:ipc';
import * as voltBench from 'volt:bench';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';
import { getAnalyticsProfile, runAnalyticsBenchmark } from './backend-logic.js';

const RESULT_FILE = '.volt-benchmark-result.json';

ipcMain.handle('analytics:profile', async (payload: unknown) => {
  const request = (payload as { datasetSize?: unknown; engine?: unknown } | null) ?? {};
  const datasetSize = Number(request.datasetSize ?? 24_000);
  const normalizedDatasetSize = Number.isFinite(datasetSize) ? datasetSize : 24_000;
  if (request.engine === 'native') {
    return voltBench.analyticsProfile({ datasetSize: normalizedDatasetSize });
  }
  return getAnalyticsProfile(normalizedDatasetSize);
});

ipcMain.handle('analytics:run', async (payload: unknown) => {
  const request = (payload as Record<string, unknown> | null) ?? {};
  if (request.engine === 'native') {
    return voltBench.runAnalyticsBenchmark(request);
  }
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
