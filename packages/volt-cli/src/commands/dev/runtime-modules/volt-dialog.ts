import { dialog } from 'voltkit';

interface DialogOptions {
  title?: string;
  defaultPath?: string;
  filters?: { name: string; extensions: string[] }[];
  multiple?: boolean;
  directory?: boolean;
}

interface MessageDialogOptions {
  dialogType?: 'info' | 'warning' | 'error';
  title?: string;
  message: string;
  buttons?: string[];
}

interface GrantDialogResult {
  paths: string[];
  grantIds: string[];
}

export async function showOpen(options: DialogOptions = {}): Promise<string | null> {
  const result = await dialog.showOpenDialog({
    title: options.title,
    defaultPath: options.defaultPath,
    filters: options.filters,
    multiSelections: options.multiple,
    directory: options.directory,
  });
  if (result.canceled || result.filePaths.length === 0) {
    return null;
  }
  return result.filePaths[0] ?? null;
}

export async function showSave(options: DialogOptions = {}): Promise<string | null> {
  const result = await dialog.showSaveDialog({
    title: options.title,
    defaultPath: options.defaultPath,
    filters: options.filters,
  });
  if (result.canceled || !result.filePath) {
    return null;
  }
  return result.filePath;
}

export async function showMessage(options: MessageDialogOptions): Promise<0 | 1> {
  const result = await dialog.showMessageBox({
    type: options.dialogType,
    title: options.title,
    message: options.message,
    buttons: options.buttons,
  });
  return result.confirmed ? 1 : 0;
}

export async function showOpenWithGrant(
  options: DialogOptions = {},
): Promise<GrantDialogResult> {
  const result = await dialog.showOpenDialog({
    title: options.title,
    defaultPath: options.defaultPath,
    filters: options.filters,
    directory: true,
    grantFsScope: true,
  });
  if (result.canceled || !result.scopeGrants) {
    return { paths: [], grantIds: [] };
  }
  return {
    paths: result.filePaths,
    grantIds: result.scopeGrants.map((g) => g.id),
  };
}

