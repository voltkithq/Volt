import { ipcMain } from 'volt:ipc';
import { runWorkflowBenchmark } from './workflow.js';

ipcMain.handle('workflow:run', (payload: unknown) => {
  return runWorkflowBenchmark((payload as Record<string, unknown> | null) ?? {});
});
