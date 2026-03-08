import { shell } from 'voltkit';

export async function openExternal(url: string): Promise<void> {
  await shell.openExternal(url);
}

export function showItemInFolder(_path: string): void {
  throw new Error('showItemInFolder is not yet supported in dev mode.');
}

