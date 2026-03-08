/**
 * Sandboxed file system module.
 * All paths are relative to the application's allowed scope.
 * No absolute paths are accepted from the frontend.
 * API methods return Promises for compatibility, but native fs operations are synchronous
 * and execute on the calling thread.
 * Requires `permissions: ['fs']` in volt.config.ts.
 */

import {
  fsReadFile,
  fsReadFileText,
  fsWriteFile,
  fsReadDir,
  fsStat,
  fsMkdir,
  fsRemove,
} from '@voltkit/volt-native';

/** File metadata returned by stat(). */
export interface FileInfo {
  /** File size in bytes. */
  size: number;
  /** Whether the path is a file. */
  isFile: boolean;
  /** Whether the path is a directory. */
  isDir: boolean;
  /** Whether the file is read-only. */
  readonly: boolean;
}

/**
 * The base directory for sandboxed file operations.
 * Defaults to process.cwd(). Set via {@link setBaseDir} during app initialization.
 */
let baseDir = process.cwd();

/**
 * Set the base directory for all sandboxed file operations.
 * Called internally during app initialization based on volt.config.ts scope.
 * @internal
 */
export function setBaseDir(dir: string): void {
  baseDir = dir;
}

/**
 * Read a file as a UTF-8 string.
 * Path must be relative to the app scope.
 *
 * @example
 * ```ts
 * const content = await fs.readFile('data/config.json');
 * ```
 */
async function readFile(path: string): Promise<string> {
  validatePath(path);
  return fsReadFileText(baseDir, path);
}

/** Read a file as raw bytes. */
async function readFileBinary(path: string): Promise<Uint8Array> {
  validatePath(path);
  const buf = fsReadFile(baseDir, path);
  return new Uint8Array(buf);
}

/** Write a string to a file, creating it if it doesn't exist. */
async function writeFile(path: string, data: string): Promise<void> {
  validatePath(path);
  fsWriteFile(baseDir, path, Buffer.from(data, 'utf-8'));
}

/** Write raw bytes to a file. */
async function writeFileBinary(path: string, data: Uint8Array): Promise<void> {
  validatePath(path);
  fsWriteFile(baseDir, path, Buffer.from(data));
}

/** List entries in a directory. Returns an array of file/directory names. */
async function readDir(path: string): Promise<string[]> {
  validatePath(path);
  return fsReadDir(baseDir, path);
}

/** Get metadata for a file or directory. */
async function stat(path: string): Promise<FileInfo> {
  validatePath(path);
  const info = fsStat(baseDir, path);
  return {
    size: info.size,
    isFile: info.isFile,
    isDir: info.isDir,
    readonly: info.readonly,
  };
}

/** Create a directory (and parent directories if needed). */
async function mkdir(path: string): Promise<void> {
  validatePath(path);
  fsMkdir(baseDir, path);
}

/** Remove a file or directory (directories are removed recursively by native layer). */
async function remove(path: string): Promise<void> {
  validatePath(path);
  fsRemove(baseDir, path);
}

/**
 * Validate that a path is safe (no absolute paths, no traversal).
 * This is a TypeScript-side guard; the Rust side also validates.
 */
function validatePath(path: string): void {
  if (path === '') {
    throw new Error('Path cannot be empty.');
  }

  if (path.includes('\0')) {
    throw new Error(`Null bytes are not allowed in paths: "${path}".`);
  }

  // Reject absolute paths
  if (path.startsWith('/') || path.startsWith('\\') || /^[a-zA-Z]:/.test(path)) {
    throw new Error(
      `Absolute paths are not allowed: "${path}". Use paths relative to the app scope.`,
    );
  }

  // Reject path traversal by segments only.
  const segments = path.split(/[\\/]+/).filter(Boolean);
  if (segments.includes('..')) {
    throw new Error(`Path traversal is not allowed: "${path}".`);
  }
}

/** Sandboxed file system APIs. Requires `permissions: ['fs']` in volt.config.ts. */
export const fs = {
  readFile,
  readFileBinary,
  writeFile,
  writeFileBinary,
  readDir,
  stat,
  mkdir,
  remove,
};
