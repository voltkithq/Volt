/**
 * File Explorer — backend
 *
 * Demonstrates the full grant-token flow:
 *   1. Open a native folder picker with `showOpenWithGrant`
 *   2. Receive a grant ID (opaque, scoped to that directory)
 *   3. Call `bindScope(grantId)` to get a ScopedFs handle
 *   4. Use the handle for all file operations (read, write, watch)
 *
 * All paths are relative to the granted directory — absolute paths
 * and traversal (`..`) are rejected at both the TypeScript and Rust layers.
 */

import { ipcMain } from 'volt:ipc';
import { bindScope, type ScopedFs } from 'volt:fs';
import { showOpenWithGrant } from 'volt:dialog';

let scopedFs: ScopedFs | null = null;

// ── Pick a folder and bind scope ─────────────────────────────────

ipcMain.handle('folder:pick', async () => {
  const result = await showOpenWithGrant({ title: 'Open Folder' });
  if (!result.grantIds.length) return { ok: false };

  scopedFs = await bindScope(result.grantIds[0]);
  return { ok: true, path: result.paths[0] ?? '' };
});

// ── List directory contents ──────────────────────────────────────

ipcMain.handle('folder:list', async (args: { path: string }) => {
  if (!scopedFs) throw new Error('No folder open');

  const entries = await scopedFs.readDir(args.path);
  const items = [];

  for (const name of entries.sort()) {
    const entryPath = args.path ? `${args.path}/${name}` : name;
    const info = await scopedFs.stat(entryPath);
    items.push({
      name,
      path: entryPath,
      isDir: info.isDir,
      size: info.size,
      modifiedMs: info.modifiedMs,
    });
  }

  return items;
});

// ── Read file contents ───────────────────────────────────────────

ipcMain.handle('file:read', async (args: { path: string }) => {
  if (!scopedFs) throw new Error('No folder open');
  return await scopedFs.readFile(args.path);
});

// ── Write file contents ──────────────────────────────────────────

ipcMain.handle('file:write', async (args: { path: string; content: string }) => {
  if (!scopedFs) throw new Error('No folder open');
  await scopedFs.writeFile(args.path, args.content);
  return { ok: true };
});

// ── Watch for changes ────────────────────────────────────────────

let watcherActive = false;

ipcMain.handle('watch:start', async () => {
  if (!scopedFs) throw new Error('No folder open');

  const watcher = await scopedFs.watch('', { recursive: true });
  watcherActive = true;

  // Poll loop — emits events to the renderer
  (async () => {
    while (watcherActive) {
      const events = await watcher.poll();
      if (events.length > 0) {
        ipcMain.emit('watch:events', events);
      }
      // Small delay between polls
      await new Promise((r) => setTimeout(r, 500));
    }
    await watcher.close();
  })();

  return { ok: true };
});

ipcMain.handle('watch:stop', () => {
  watcherActive = false;
  return { ok: true };
});
