import { ipcMain } from 'volt:ipc';

ipcMain.handle('app:ping', () => ({ ok: true }));
