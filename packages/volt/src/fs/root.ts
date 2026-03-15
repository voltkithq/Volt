import {
  fsExists,
  fsMkdir,
  fsReadDir,
  fsReadFile,
  fsReadFileText,
  fsRemove,
  fsStat,
  fsWriteFile,
} from '@voltkit/volt-native';

import { getBaseDir } from './state.js';
import type { FileInfo } from './types.js';
import { validatePath } from './validation.js';

export async function readFile(path: string): Promise<string> {
  validatePath(path);
  return fsReadFileText(getBaseDir(), path);
}

export async function readFileBinary(path: string): Promise<Uint8Array> {
  validatePath(path);
  return new Uint8Array(fsReadFile(getBaseDir(), path));
}

export async function writeFile(path: string, data: string): Promise<void> {
  validatePath(path);
  fsWriteFile(getBaseDir(), path, Buffer.from(data, 'utf-8'));
}

export async function writeFileBinary(path: string, data: Uint8Array): Promise<void> {
  validatePath(path);
  fsWriteFile(getBaseDir(), path, Buffer.from(data));
}

export async function readDir(path: string): Promise<string[]> {
  validatePath(path);
  return fsReadDir(getBaseDir(), path);
}

export async function stat(path: string): Promise<FileInfo> {
  validatePath(path);
  const info = fsStat(getBaseDir(), path);
  return {
    size: info.size,
    isFile: info.isFile,
    isDir: info.isDir,
    readonly: info.readonly,
    modifiedMs: info.modifiedMs,
    createdMs: info.createdMs ?? null,
  };
}

export async function exists(path: string): Promise<boolean> {
  validatePath(path);
  return fsExists(getBaseDir(), path);
}

export async function mkdir(path: string): Promise<void> {
  validatePath(path);
  fsMkdir(getBaseDir(), path);
}

export async function remove(path: string): Promise<void> {
  validatePath(path);
  fsRemove(getBaseDir(), path);
}
