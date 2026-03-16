import { shell } from 'voltkit';
import { ensureDevPermission } from './shared.js';

export async function openExternal(url: string): Promise<void> {
  ensureDevPermission('shell', 'shell.openExternal()');
  await shell.openExternal(url);
}

export function showItemInFolder(path: string): void {
  ensureDevPermission('shell', 'shell.showItemInFolder()');
  shell.showItemInFolder(path);
}
