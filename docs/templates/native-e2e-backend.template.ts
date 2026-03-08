import { ipcMain } from 'volt:ipc';
import * as voltDialog from 'volt:dialog';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';

const RESULT_FILE = '.volt-smoke-result.json';

ipcMain.handle('e2e:status', async () => {
  const windowCount = await voltWindow.getWindowCount();
  return {
    runtime: {
      windowCount,
      nativeReady: true,
    },
  };
});

ipcMain.handle('e2e:dialog:open', async () => {
  return voltDialog.showOpenDialog({
    title: 'Select File',
    multiSelections: false,
  });
});

ipcMain.handle('e2e:complete', async (payload: unknown) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
