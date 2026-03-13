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
  fsExists,
  fsMkdir,
  fsRemove,
  fsResolveGrant,
  fsRename,
  fsCopy,
  fsWatchStart,
  fsWatchPoll,
  fsWatchClose,
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
  /** Last modification time as milliseconds since Unix epoch. */
  modifiedMs: number;
  /** Creation time as milliseconds since Unix epoch, or null if unavailable. */
  createdMs: number | null;
}

/** A file system watch event. */
export interface WatchEvent {
  /** Event kind. */
  kind: 'create' | 'change' | 'delete' | 'rename' | 'overflow';
  /** Scope-relative path of the affected file/directory. */
  path: string;
  /** For rename events, the old scope-relative path (if available). */
  oldPath?: string;
  /** Whether the event target is a directory. */
  isDir?: boolean;
}

/** Options for starting a file watcher. */
export interface WatchOptions {
  /** Watch subdirectories recursively. Defaults to true. */
  recursive?: boolean;
  /** Debounce interval in milliseconds. Defaults to 200. */
  debounceMs?: number;
  /** Optional callback invoked when events occur. Uses internal polling at the debounce interval. */
  onEvent?: (events: WatchEvent[]) => void;
}

/** A file watcher handle. Call poll() to retrieve events, close() to stop watching. */
export interface FileWatcher {
  /** Drain all pending events since the last poll. */
  poll(): Promise<WatchEvent[]>;
  /** Register a callback for file change events. Starts an internal polling loop. */
  on(event: 'change', handler: (events: WatchEvent[]) => void): void;
  /** Remove a previously registered callback. */
  off(event: 'change', handler: (events: WatchEvent[]) => void): void;
  /** Stop watching and release resources. */
  close(): Promise<void>;
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
    modifiedMs: info.modifiedMs,
    createdMs: info.createdMs ?? null,
  };
}

/**
 * Check whether a path exists within the app scope.
 *
 * @example
 * ```ts
 * if (await fs.exists('data/config.json')) { ... }
 * ```
 */
async function exists(path: string): Promise<boolean> {
  validatePath(path);
  return fsExists(baseDir, path);
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

/** A scoped file handle bound to a grant. */
export interface ScopedFs {
  readFile(path: string): Promise<string>;
  readFileBinary(path: string): Promise<Uint8Array>;
  readDir(path: string): Promise<string[]>;
  stat(path: string): Promise<FileInfo>;
  exists(path: string): Promise<boolean>;
  writeFile(path: string, data: string): Promise<void>;
  writeFileBinary(path: string, data: Uint8Array): Promise<void>;
  mkdir(path: string): Promise<void>;
  remove(path: string): Promise<void>;
  rename(from: string, to: string): Promise<void>;
  copy(from: string, to: string): Promise<void>;
  watch(subpath: string, options?: WatchOptions): Promise<FileWatcher>;
}

/**
 * Bind a filesystem scope grant to create a scoped handle.
 * The grant must have been created by a `showOpenDialog({ grantFsScope: true })` call.
 *
 * @example
 * ```ts
 * import { bindScope } from 'voltkit';
 * const scopedFs = await bindScope(grantId);
 * const entries = await scopedFs.readDir('');
 * await scopedFs.writeFile('notes/new.md', '# New Note');
 * ```
 */
async function bindScope(grantId: string): Promise<ScopedFs> {
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
      const buf = fsReadFile(grantBasePath, path);
      return new Uint8Array(buf);
    },
    async readDir(path: string): Promise<string[]> {
      validateScopedPath(path);
      return fsReadDir(grantBasePath, path);
    },
    async stat(path: string): Promise<FileInfo> {
      validateScopedPath(path);
      const info = fsStat(grantBasePath, path);
      return {
        size: info.size,
        isFile: info.isFile,
        isDir: info.isDir,
        readonly: info.readonly,
        modifiedMs: info.modifiedMs,
        createdMs: info.createdMs ?? null,
      };
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
    async watch(subpath: string, options?: WatchOptions): Promise<FileWatcher> {
      validateScopedPath(subpath);
      const recursive = options?.recursive ?? true;
      const debounceMs = options?.debounceMs ?? 200;
      const watcherId = fsWatchStart(grantBasePath, subpath, recursive, debounceMs);
      const handlers = new Set<(events: WatchEvent[]) => void>();
      let pollInterval: ReturnType<typeof setInterval> | null = null;

      if (options?.onEvent) {
        handlers.add(options.onEvent);
      }

      function startPolling(): void {
        if (pollInterval) return;
        pollInterval = setInterval(() => {
          const events = fsWatchPoll(watcherId) as WatchEvent[];
          if (events.length > 0) {
            for (const handler of handlers) {
              try {
                handler(events);
              } catch {
                /* handler errors should not crash the watcher */
              }
            }
          }
        }, Math.max(debounceMs, 50));
      }

      function stopPollingIfEmpty(): void {
        if (handlers.size === 0 && pollInterval) {
          clearInterval(pollInterval);
          pollInterval = null;
        }
      }

      if (handlers.size > 0) startPolling();

      return {
        async poll(): Promise<WatchEvent[]> {
          return fsWatchPoll(watcherId) as WatchEvent[];
        },
        on(event: 'change', handler: (events: WatchEvent[]) => void): void {
          if (event === 'change') {
            handlers.add(handler);
            startPolling();
          }
        },
        off(event: 'change', handler: (events: WatchEvent[]) => void): void {
          if (event === 'change') {
            handlers.delete(handler);
            stopPollingIfEmpty();
          }
        },
        async close(): Promise<void> {
          if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
          }
          handlers.clear();
          fsWatchClose(watcherId);
        },
      };
    },
  };
}

/**
 * Resolve a grant ID to its root path using the native grant store.
 * Throws if the grant is invalid or expired.
 */
function resolveGrantPath(grantId: string): string {
  // The native grant store is in Rust. We need a way to look up the path.
  // For the N-API layer, we use a dedicated native function.
  return fsResolveGrant(grantId);
}

/**
 * Validate a scoped path. Unlike validatePath, this allows empty strings
 * (to reference the scope root directory itself for readDir/stat/exists).
 */
function validateScopedPath(path: string): void {
  if (path === '') return; // empty = scope root, valid for readDir/stat/exists
  validatePath(path);
}

/**
 * Watch a directory for file changes within the app scope.
 *
 * @example
 * ```ts
 * const watcher = await fs.watch('data', { recursive: true });
 * // Later...
 * const events = await watcher.poll();
 * await watcher.close();
 * ```
 */
async function watch(path: string, options?: WatchOptions): Promise<FileWatcher> {
  validatePath(path);
  const recursive = options?.recursive ?? true;
  const debounceMs = options?.debounceMs ?? 200;
  const watcherId = fsWatchStart(baseDir, path, recursive, debounceMs);
  const handlers = new Set<(events: WatchEvent[]) => void>();
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  if (options?.onEvent) {
    handlers.add(options.onEvent);
  }

  function startPolling(): void {
    if (pollInterval) return;
    pollInterval = setInterval(() => {
      const events = fsWatchPoll(watcherId) as WatchEvent[];
      if (events.length > 0) {
        for (const handler of handlers) {
          try {
            handler(events);
          } catch {
            /* handler errors should not crash the watcher */
          }
        }
      }
    }, Math.max(debounceMs, 50));
  }

  function stopPollingIfEmpty(): void {
    if (handlers.size === 0 && pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  if (handlers.size > 0) startPolling();

  return {
    async poll(): Promise<WatchEvent[]> {
      return fsWatchPoll(watcherId) as WatchEvent[];
    },
    on(event: 'change', handler: (events: WatchEvent[]) => void): void {
      if (event === 'change') {
        handlers.add(handler);
        startPolling();
      }
    },
    off(event: 'change', handler: (events: WatchEvent[]) => void): void {
      if (event === 'change') {
        handlers.delete(handler);
        stopPollingIfEmpty();
      }
    },
    async close(): Promise<void> {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
      handlers.clear();
      fsWatchClose(watcherId);
    },
  };
}

/** Sandboxed file system APIs. Requires `permissions: ['fs']` in volt.config.ts. */
export const fs = {
  readFile,
  readFileBinary,
  writeFile,
  writeFileBinary,
  readDir,
  stat,
  exists,
  mkdir,
  remove,
  bindScope,
  watch,
};
