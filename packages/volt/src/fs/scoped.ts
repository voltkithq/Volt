import {
  fsCopy,
  fsExists,
  fsMkdir,
  fsReadDir,
  fsReadFile,
  fsReadFileText,
  fsRemove,
  fsRename,
  fsResolveGrant,
  fsStat,
  fsWriteFile,
} from '@voltkit/volt-native';

import type { FileInfo, ScopedFs, WatchOptions } from './types.js';
import { validatePath, validateScopedPath } from './validation.js';
import { createWatcher } from './watcher.js';

/**
 * Resolve a grant ID to its root path using the native grant store.
 * Throws if the grant is invalid or expired.
 */
function resolveGrantPath(grantId: string): string {
  return fsResolveGrant(grantId);
}

function mapFileInfo(info: {
  size: number;
  isFile: boolean;
  isDir: boolean;
  readonly: boolean;
  modifiedMs: number;
  createdMs?: number | null;
}): FileInfo {
  return {
    size: info.size,
    isFile: info.isFile,
    isDir: info.isDir,
    readonly: info.readonly,
    modifiedMs: info.modifiedMs,
    createdMs: info.createdMs ?? null,
  };
}

/**
 * Bind a filesystem scope grant to create a scoped handle.
 * The grant must have been created by a `showOpenDialog({ grantFsScope: true })` call.
 */
export async function bindScope(grantId: string): Promise<ScopedFs> {
  if (!grantId || typeof grantId !== 'string') {
    throw new Error('FS_SCOPE_INVALID: grant ID must be a non-empty string');
  }

  const grantBasePath = resolveGrantPath(grantId);

  return {
    async readFile(path: string): Promise<string> {
      validatePath(path);
      return fsReadFileText(grantBasePath, path);
    },
    async readFileBinary(path: string): Promise<Uint8Array> {
      validatePath(path);
      return new Uint8Array(fsReadFile(grantBasePath, path));
    },
    async readDir(path: string): Promise<string[]> {
      validateScopedPath(path);
      return fsReadDir(grantBasePath, path);
    },
    async stat(path: string): Promise<FileInfo> {
      validateScopedPath(path);
      return mapFileInfo(fsStat(grantBasePath, path));
    },
    async exists(path: string): Promise<boolean> {
      validateScopedPath(path);
      return fsExists(grantBasePath, path);
    },
    async writeFile(path: string, data: string): Promise<void> {
      validatePath(path);
      fsWriteFile(grantBasePath, path, Buffer.from(data, 'utf-8'));
    },
    async writeFileBinary(path: string, data: Uint8Array): Promise<void> {
      validatePath(path);
      fsWriteFile(grantBasePath, path, Buffer.from(data));
    },
    async mkdir(path: string): Promise<void> {
      validatePath(path);
      fsMkdir(grantBasePath, path);
    },
    async remove(path: string): Promise<void> {
      validatePath(path);
      fsRemove(grantBasePath, path);
    },
    async rename(from: string, to: string): Promise<void> {
      validatePath(from);
      validatePath(to);
      fsRename(grantBasePath, from, to);
    },
    async copy(from: string, to: string): Promise<void> {
      validatePath(from);
      validatePath(to);
      fsCopy(grantBasePath, from, to);
    },
    async watch(subpath: string, options?: WatchOptions) {
      validateScopedPath(subpath);
      return createWatcher(grantBasePath, subpath, options);
    },
  };
}
