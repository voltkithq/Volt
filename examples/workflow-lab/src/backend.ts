import { ipcMain } from 'volt:ipc';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';
import { listWorkflowPlugins } from './plugins.js';
import { runWorkflowBenchmark } from './workflow.js';

const RESULT_FILE = '.volt-benchmark-result.json';

ipcMain.handle('workflow:plugins', () => listWorkflowPlugins());

ipcMain.handle('workflow:run', (payload: unknown) => {
  return runWorkflowBenchmark((payload as Record<string, unknown> | null) ?? {});
});

ipcMain.handle('benchmark:complete', async (payload: unknown) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
