declare module 'volt:ipc' {
  export interface IpcMain {
    handle(channel: string, handler: (args: unknown) => unknown | Promise<unknown>): void;
    removeHandler(channel: string): void;
    clearHandlers(): void;
    hasHandler(channel: string): boolean;
    emit(eventName: string, data?: unknown): void;
    emitTo(windowId: string, eventName: string, data?: unknown): void;
  }

  export const ipcMain: IpcMain;
}

declare module 'volt:events' {
  export function emit(eventName: string, data?: unknown): void;
  export function emitTo(windowId: string, eventName: string, data?: unknown): void;
}

declare module 'volt:window' {
  export function close(windowId?: string): void;
  export function show(windowId?: string): void;
  export function focus(windowId?: string): void;
  export function maximize(windowId?: string): void;
  export function minimize(windowId?: string): void;
  export function restore(windowId?: string): void;
  export function getWindowCount(): Promise<number>;
  export function quit(): void;
}

declare module 'volt:menu' {
  export function setAppMenu(template: unknown): Promise<void>;
  export function on(eventName: 'click', handler: (payload: unknown) => void): void;
  export function off(eventName: 'click', handler: (payload: unknown) => void): void;
}

declare module 'volt:globalShortcut' {
  export function register(accelerator: string): Promise<number>;
  export function unregister(accelerator: string): Promise<void>;
  export function unregisterAll(): Promise<void>;
  export function on(eventName: 'triggered', handler: (payload: unknown) => void): void;
  export function off(eventName: 'triggered', handler: (payload: unknown) => void): void;
}

declare module 'volt:tray' {
  export interface TrayCreateOptions {
    tooltip?: string;
    icon?: string;
  }

  export function create(options?: TrayCreateOptions): Promise<void>;
  export function setTooltip(tooltip: string): void;
  export function setVisible(visible: boolean): void;
  export function destroy(): void;
  export function on(eventName: 'click', handler: (payload: unknown) => void): void;
  export function off(eventName: 'click', handler: (payload: unknown) => void): void;
}

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

declare module 'volt:clipboard' {
  export function readText(): string;
  export function writeText(text: string): void;
}

declare module 'volt:crypto' {
  export function sha256(data: string): string;
  export function base64Encode(data: string): string;
  export function base64Decode(data: string): string;
}

declare module 'volt:os' {
  export function platform(): string;
  export function arch(): string;
  export function homeDir(): string;
  export function tempDir(): string;
}

declare module 'volt:shell' {
  export function openExternal(url: string): Promise<void>;
  export function showItemInFolder(path: string): void;
}

declare module 'volt:notification' {
  export interface NotificationOptions {
    title: string;
    body?: string;
    icon?: string;
  }

  export function show(options: NotificationOptions): void;
}

declare module 'volt:dialog' {
  export interface FileFilter {
    name: string;
    extensions: string[];
  }

  export interface OpenDialogOptions {
    title?: string;
    defaultPath?: string;
    filters?: FileFilter[];
    multiple?: boolean;
    directory?: boolean;
  }

  export interface SaveDialogOptions {
    title?: string;
    defaultPath?: string;
    filters?: FileFilter[];
  }

  export interface MessageDialogOptions {
    dialogType?: 'info' | 'warning' | 'error';
    title?: string;
    message: string;
    buttons?: string[];
  }

  export interface GrantDialogResult {
    paths: string[];
    grantIds: string[];
  }

  export function showOpen(options?: OpenDialogOptions): Promise<string | null>;
  export function showSave(options?: SaveDialogOptions): Promise<string | null>;
  export function showMessage(options: MessageDialogOptions): Promise<0 | 1>;
  export function showOpenWithGrant(options?: OpenDialogOptions): Promise<GrantDialogResult>;
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
  }

  export function readFile(path: string): Promise<string>;
  export function writeFile(path: string, data: string): Promise<void>;
  export function readDir(path: string): Promise<string[]>;
  export function stat(path: string): Promise<FileInfo>;
  export function exists(path: string): Promise<boolean>;
  export function mkdir(path: string): Promise<void>;
  export function remove(path: string): Promise<void>;

  /** Validate a grant and return a scoped filesystem handle. */
  export function bindScope(grantId: string): Promise<ScopedFs>;

  /** Scoped read operations — use the grant ID from bindScope(). */
  export function scopedReadFile(grantId: string, path: string): Promise<string>;
  export function scopedReadFileBinary(grantId: string, path: string): Promise<Uint8Array>;
  export function scopedReadDir(grantId: string, path: string): Promise<string[]>;
  export function scopedStat(grantId: string, path: string): Promise<FileInfo>;
  export function scopedExists(grantId: string, path: string): Promise<boolean>;

  /** Scoped write operations — use the grant ID from bindScope(). */
  export function scopedWriteFile(grantId: string, path: string, data: string): Promise<void>;
  export function scopedMkdir(grantId: string, path: string): Promise<void>;
  export function scopedRemove(grantId: string, path: string): Promise<void>;
  export function scopedRename(grantId: string, from: string, to: string): Promise<void>;
  export function scopedCopy(grantId: string, from: string, to: string): Promise<void>;

  /** Watch operations. */
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

declare module 'volt:bench' {
  export interface AnalyticsProfileOptions {
    datasetSize?: number;
  }

  export interface AnalyticsProfile {
    datasetSize: number;
    cachedSizes: number[];
    categorySpread: Record<string, number>;
    regionSpread: Record<string, number>;
  }

  export interface AnalyticsBenchmarkOptions {
    datasetSize?: number;
    iterations?: number;
    searchTerm?: string;
    minScore?: number;
    topN?: number;
  }

  export interface AnalyticsBenchmarkResult {
    datasetSize: number;
    iterations: number;
    query: string;
    minScore: number;
    topN: number;
    backendDurationMs: number;
    filterDurationMs: number;
    sortDurationMs: number;
    aggregateDurationMs: number;
    peakMatches: number;
    totalMatchesAcrossIterations: number;
    categoryWinners: Array<{ category: string; total: number }>;
    sample: Array<{
      id: number;
      title: string;
      category: string;
      region: string;
      score: number;
      revenue: number;
      margin: number;
    }>;
    payloadBytes: number;
  }

  export interface WorkflowBenchmarkOptions {
    batchSize?: number;
    passes?: number;
    pipeline?: string[];
  }

  export interface WorkflowBenchmarkResult {
    batchSize: number;
    passes: number;
    pipeline: string[];
    backendDurationMs: number;
    stepTimings: Array<{ plugin: string; durationMs: number }>;
    routeDistribution: Record<string, number>;
    averagePriority: number;
    digestSample: string[];
    payloadBytes: number;
  }

  export function analyticsProfile(options?: AnalyticsProfileOptions): Promise<AnalyticsProfile>;
  export function runAnalyticsBenchmark(options?: AnalyticsBenchmarkOptions): Promise<AnalyticsBenchmarkResult>;
  export function runWorkflowBenchmark(options?: WorkflowBenchmarkOptions): Promise<WorkflowBenchmarkResult>;
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
