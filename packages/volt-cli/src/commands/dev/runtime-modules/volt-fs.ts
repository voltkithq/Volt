import { fs as frameworkFs } from 'voltkit';
import type { FileInfo, ScopedFs, WatchEvent, FileWatcher } from 'voltkit';

export async function readFile(path: string): Promise<string> {
  return frameworkFs.readFile(path);
}

export async function writeFile(path: string, data: string): Promise<void> {
  await frameworkFs.writeFile(path, data);
}

export async function readDir(path: string): Promise<string[]> {
  return frameworkFs.readDir(path);
}

export async function stat(path: string): Promise<FileInfo> {
  return frameworkFs.stat(path);
}

export async function exists(path: string): Promise<boolean> {
  return frameworkFs.exists(path);
}

export async function mkdir(path: string): Promise<void> {
  await frameworkFs.mkdir(path);
}

export async function remove(path: string): Promise<void> {
  await frameworkFs.remove(path);
}

export async function bindScope(grantId: string): Promise<string> {
  const scopedFs = await frameworkFs.bindScope(grantId);
  // In dev mode, store the scoped handle and return the grant ID
  devScopedHandles.set(grantId, scopedFs);
  return grantId;
}

// Dev-mode store for scoped handles
const devScopedHandles = new Map<string, ScopedFs>();

export async function scopedReadFile(grantId: string, path: string): Promise<string> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  return handle.readFile(path);
}

export async function scopedReadFileBinary(grantId: string, path: string): Promise<Uint8Array> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  return handle.readFileBinary(path);
}

export async function scopedReadDir(grantId: string, path: string): Promise<string[]> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  return handle.readDir(path);
}

export async function scopedStat(grantId: string, path: string): Promise<FileInfo> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  return handle.stat(path);
}

export async function scopedExists(grantId: string, path: string): Promise<boolean> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  return handle.exists(path);
}

export async function scopedWriteFile(grantId: string, path: string, data: string): Promise<void> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  await handle.writeFile(path, data);
}

export async function scopedMkdir(grantId: string, path: string): Promise<void> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  await handle.mkdir(path);
}

export async function scopedRemove(grantId: string, path: string): Promise<void> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  await handle.remove(path);
}

export async function scopedRename(grantId: string, from: string, to: string): Promise<void> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  await handle.rename(from, to);
}

export async function scopedCopy(grantId: string, from: string, to: string): Promise<void> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  await handle.copy(from, to);
}

export async function watchStart(
  path: string,
  recursive: boolean,
  debounceMs: number,
): Promise<string> {
  const watcher = await frameworkFs.watch(path, { recursive, debounceMs });
  const id = `dev_watcher_${Date.now()}_${Math.random().toString(36).slice(2)}`;
  devWatcherHandles.set(id, watcher);
  return id;
}

export async function watchPoll(watcherId: string): Promise<WatchEvent[]> {
  const watcher = devWatcherHandles.get(watcherId);
  if (!watcher) throw new Error('watcher not found');
  return watcher.poll();
}

export async function watchClose(watcherId: string): Promise<void> {
  const watcher = devWatcherHandles.get(watcherId);
  if (!watcher) throw new Error('watcher not found');
  await watcher.close();
  devWatcherHandles.delete(watcherId);
}

export async function scopedWatchStart(
  grantId: string,
  subpath: string,
  recursive: boolean,
  debounceMs: number,
): Promise<string> {
  const handle = devScopedHandles.get(grantId);
  if (!handle) throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
  const watcher = await handle.watch(subpath, { recursive, debounceMs });
  const id = `dev_watcher_${Date.now()}_${Math.random().toString(36).slice(2)}`;
  devWatcherHandles.set(id, watcher);
  return id;
}

export async function scopedWatchPoll(watcherId: string): Promise<WatchEvent[]> {
  return watchPoll(watcherId);
}

export async function scopedWatchClose(watcherId: string): Promise<void> {
  return watchClose(watcherId);
}

// Dev-mode store for watcher handles
const devWatcherHandles = new Map<string, FileWatcher>();

