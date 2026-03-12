import { ipcMain } from 'volt:ipc';
import * as voltBench from 'volt:bench';
import { runWorkflowBenchmark } from './workflow.js';

ipcMain.handle('workflow:run', async (payload: unknown) => {
  const request = (payload as Record<string, unknown> | null) ?? {};
  if (request.engine === 'native') {
    return voltBench.runWorkflowBenchmark(request);
  }
  return runWorkflowBenchmark(request);
});
