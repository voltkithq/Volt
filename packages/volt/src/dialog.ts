/**
 * Native dialog module.
 * Provides file open/save dialogs and message boxes.
 * API methods return Promises for compatibility, but underlying native calls are synchronous
 * and block until the OS dialog closes.
 * Requires `permissions: ['dialog']` in volt.config.ts.
 */

import {
  type NativeMessageDialogOptions,
  type NativeOpenDialogOptions,
  type NativeSaveDialogOptions,
  dialogShowOpen,
  dialogShowOpenWithGrant,
  dialogShowSave,
  dialogShowMessage,
} from '@voltkit/volt-native';

/** File type filter for open/save dialogs. */
export interface FileFilter {
  /** Display name for this filter (e.g., 'Images'). */
  name: string;
  /** File extensions without dots (e.g., ['png', 'jpg']). */
  extensions: string[];
}

/** Options for the open file dialog. */
export interface OpenDialogOptions {
  /** Dialog window title. */
  title?: string;
  /** Default starting directory. */
  defaultPath?: string;
  /** File type filters. */
  filters?: FileFilter[];
  /** Allow selecting multiple files. Default: false. */
  multiSelections?: boolean;
  /** Allow selecting directories instead of files. Default: false. */
  directory?: boolean;
  /** When true, creates filesystem scope grants for selected directories.
   *  Forces directory mode. Requires both `dialog` and `fs` permissions. */
  grantFsScope?: boolean;
}

/** A filesystem scope grant created by a grant-aware dialog. */
export interface FileScopeGrant {
  /** Opaque grant ID. Pass this to backend via IPC for `bindScope()`. */
  id: string;
  /** The kind of grant (always 'directory' in v1). */
  kind: 'directory';
}

/** Result from showOpenDialog. */
export interface OpenDialogResult {
  /** Whether the dialog was cancelled. */
  canceled: boolean;
  /** Selected file paths (empty if cancelled). */
  filePaths: string[];
  /** Scope grants for selected directories (only present when grantFsScope was true). */
  scopeGrants?: FileScopeGrant[];
}

/** Options for the save file dialog. */
export interface SaveDialogOptions {
  /** Dialog window title. */
  title?: string;
  /** Default file path/name. */
  defaultPath?: string;
  /** File type filters. */
  filters?: FileFilter[];
}

/** Result from showSaveDialog. */
export interface SaveDialogResult {
  /** Whether the dialog was cancelled. */
  canceled: boolean;
  /** Selected file path (empty string if cancelled). */
  filePath: string;
}

/** Options for message box dialogs. */
export interface MessageBoxOptions {
  /** Message box type. Default: 'info'. */
  type?: 'info' | 'warning' | 'error';
  /** Dialog title. */
  title?: string;
  /** Dialog message. */
  message: string;
  /** Button labels. */
  buttons?: string[];
}

/** Result from showMessageBox. */
export interface MessageBoxResult {
  /** Whether the user confirmed (clicked OK/Yes). */
  confirmed: boolean;
}

/**
 * Show a native open file dialog.
 * Electron-compatible API.
 * Note: this call blocks the current thread in native code.
 *
 * @example
 * ```ts
 * const result = await dialog.showOpenDialog({
 *   title: 'Select Image',
 *   filters: [{ name: 'Images', extensions: ['png', 'jpg'] }],
 * });
 * if (!result.canceled) {
 *   console.log(result.filePaths);
 * }
 * ```
 */
async function showOpenDialog(
  options: OpenDialogOptions = {},
): Promise<OpenDialogResult> {
  // Map camelCase TS options to snake_case for Rust serde
  const nativeOpts: NativeOpenDialogOptions = {
    title: options.title,
    default_path: options.defaultPath,
    filters: options.filters,
    multiple: options.multiSelections ?? false,
    directory: options.directory ?? false,
  };

  if (options.grantFsScope) {
    // Use grant-aware dialog — forces directory mode
    const result = dialogShowOpenWithGrant(nativeOpts);
    const scopeGrants: FileScopeGrant[] = result.grantIds.map((id) => ({
      id,
      kind: 'directory' as const,
    }));
    return {
      canceled: result.paths.length === 0,
      filePaths: result.paths,
      scopeGrants,
    };
  }

  const filePaths = dialogShowOpen(nativeOpts);
  return {
    canceled: filePaths.length === 0,
    filePaths,
  };
}

/** Show a native save file dialog. Note: this call blocks the current thread in native code. */
async function showSaveDialog(
  options: SaveDialogOptions = {},
): Promise<SaveDialogResult> {
  const nativeOpts: NativeSaveDialogOptions = {
    title: options.title,
    default_path: options.defaultPath,
    filters: options.filters,
  };

  const filePath = dialogShowSave(nativeOpts);
  return {
    canceled: filePath === null,
    filePath: filePath ?? '',
  };
}

/** Show a native message box dialog. Note: this call blocks the current thread in native code. */
async function showMessageBox(
  options: MessageBoxOptions,
): Promise<MessageBoxResult> {
  const nativeOpts: NativeMessageDialogOptions = {
    dialog_type: options.type ?? 'info',
    title: options.title ?? '',
    message: options.message,
    buttons: options.buttons ?? [],
  };

  const confirmed = dialogShowMessage(nativeOpts);
  return { confirmed };
}

/** Native dialog APIs. Requires `permissions: ['dialog']` in volt.config.ts. */
export const dialog = {
  showOpenDialog,
  showSaveDialog,
  showMessageBox,
};
