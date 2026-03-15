declare module 'volt:db' {
  export interface DbExecuteResult {
    rowsAffected: number;
  }

  export function open(path: string): Promise<void>;
  export function close(): Promise<void>;
  export function execute(sql: string, params?: unknown[]): Promise<DbExecuteResult>;
  export function query(sql: string, params?: unknown[]): Promise<unknown[]>;
  export function queryOne(sql: string, params?: unknown[]): Promise<unknown | null>;
  export function transaction<T>(callback: () => Promise<T> | T): Promise<T>;
}

declare module 'volt:secureStorage' {
  export function set(key: string, value: string): Promise<void>;
  export function get(key: string): Promise<string | null>;
  function remove(key: string): Promise<void>;
  export { remove as delete };
  export function has(key: string): Promise<boolean>;
}

declare module 'volt:fs' {
  export interface FileInfo {
    size: number;
    isFile: boolean;
    isDir: boolean;
    readonly: boolean;
    modifiedMs: number;
    createdMs: number | null;
  }

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
  }

  export function readFile(path: string): Promise<string>;
  export function writeFile(path: string, data: string): Promise<void>;
  export function readDir(path: string): Promise<string[]>;
  export function stat(path: string): Promise<FileInfo>;
  export function exists(path: string): Promise<boolean>;
  export function mkdir(path: string): Promise<void>;
  export function remove(path: string): Promise<void>;
  export function bindScope(grantId: string): Promise<ScopedFs>;
  export function scopedReadFile(grantId: string, path: string): Promise<string>;
  export function scopedReadFileBinary(grantId: string, path: string): Promise<Uint8Array>;
  export function scopedReadDir(grantId: string, path: string): Promise<string[]>;
  export function scopedStat(grantId: string, path: string): Promise<FileInfo>;
  export function scopedExists(grantId: string, path: string): Promise<boolean>;
  export function scopedWriteFile(grantId: string, path: string, data: string): Promise<void>;
  export function scopedMkdir(grantId: string, path: string): Promise<void>;
  export function scopedRemove(grantId: string, path: string): Promise<void>;
  export function scopedRename(grantId: string, from: string, to: string): Promise<void>;
  export function scopedCopy(grantId: string, from: string, to: string): Promise<void>;
  export function watchStart(path: string, recursive: boolean, debounceMs: number): Promise<string>;
  export function watchPoll(watcherId: string): Promise<unknown[]>;
  export function watchClose(watcherId: string): Promise<void>;
  export function scopedWatchStart(
    grantId: string,
    subpath: string,
    recursive: boolean,
    debounceMs: number,
  ): Promise<string>;
  export function scopedWatchPoll(watcherId: string): Promise<unknown[]>;
  export function scopedWatchClose(watcherId: string): Promise<void>;
}

declare module 'volt:http' {
  export interface HttpFetchRequest {
    url: string;
    method?: string;
    headers?: Record<string, string>;
    body?: unknown;
    timeoutMs?: number;
  }

  export interface HttpFetchResponse {
    status: number;
    headers: Record<string, string[]>;
    text(): Promise<string>;
    json(): Promise<unknown>;
  }

  export function fetch(request: HttpFetchRequest): Promise<HttpFetchResponse>;
}

declare module 'volt:updater' {
  export interface UpdateCheckOptions {
    url: string;
    currentVersion: string;
  }

  export interface UpdateInfo {
    version: string;
    url: string;
    signature: string;
    sha256: string;
  }

  export function checkForUpdate(options: UpdateCheckOptions): Promise<UpdateInfo | null>;
  export function downloadAndInstall(updateInfo: UpdateInfo): Promise<void>;
  export function cancelDownloadAndInstall(): void;
}
