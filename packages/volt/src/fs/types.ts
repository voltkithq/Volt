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
