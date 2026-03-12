import { ipcMain } from 'volt:ipc';
import * as voltBench from 'volt:bench';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';
import { listWorkflowPlugins } from './plugins.js';
import { runWorkflowBenchmark } from './workflow.js';

const RESULT_FILE = '.volt-benchmark-result.json';

ipcMain.handle('workflow:plugins', () => listWorkflowPlugins());

ipcMain.handle('workflow:run', async (payload: unknown) => {
  const request = (payload as Record<string, unknown> | null) ?? {};
  if (request.engine === 'native') {
    return voltBench.runWorkflowBenchmark(request);
  }
  return runWorkflowBenchmark(request);
});

ipcMain.handle('benchmark:complete', async (payload: unknown) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
