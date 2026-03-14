import { shell } from 'voltkit';

export async function openExternal(url: string): Promise<void> {
  await shell.openExternal(url);
}

export function showItemInFolder(path: string): void {
  shell.showItemInFolder(path);
}

