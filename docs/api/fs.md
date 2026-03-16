# File System

Sandboxed file operations. Requires `permissions: ['fs']`.

All paths must be **relative** to the application's scope. Absolute paths, path traversal (`..`), and Windows reserved device names are rejected.

## Scoped Access

The recommended pattern for user-facing file access is **grant tokens + scoped handles**. This lets users pick a folder via a dialog, which returns a grant token that your backend uses to create a sandboxed filesystem handle.

```ts
// backend.ts
import { showOpenWithGrant } from 'volt:dialog';
import { bindScope } from 'volt:fs';
import { ipcMain } from 'volt:ipc';

let scopedFs: ScopedFs | null = null;

ipcMain.handle('open-folder', async () => {
  const result = await showOpenWithGrant({ title: 'Open Folder' });
  if (!result.grantIds.length) return { ok: false };

  // Bind a scoped handle — all ops are confined to this directory
  scopedFs = await bindScope(result.grantIds[0]);

  const files = await scopedFs.readDir('');
  return { ok: true, files };
});

ipcMain.handle('read-file', async (args: { path: string }) => {
  if (!scopedFs) throw new Error('No folder open');
  return await scopedFs.readFile(args.path);
});
```

### `bindScope(grantId): Promise<ScopedFs>`

Create a scoped filesystem handle from a grant ID (obtained via `showOpenWithGrant`).

### `ScopedFs` Interface

| Method | Returns | Description |
|--------|---------|-------------|
| `readFile(path)` | `Promise<string>` | Read file as UTF-8 |
| `readFileBinary(path)` | `Promise<Uint8Array>` | Read file as bytes |
| `readDir(path)` | `Promise<string[]>` | List directory entries |
| `stat(path)` | `Promise<FileInfo>` | Get file/directory metadata |
| `exists(path)` | `Promise<boolean>` | Check if path exists |
| `writeFile(path, data)` | `Promise<void>` | Write UTF-8 string |
| `writeFileBinary(path, data)` | `Promise<void>` | Write bytes |
| `mkdir(path)` | `Promise<void>` | Create directory (recursive) |
| `remove(path)` | `Promise<void>` | Remove file or directory |
| `rename(from, to)` | `Promise<void>` | Move/rename within scope |
| `copy(from, to)` | `Promise<void>` | Copy a file within scope |
| `watch(subpath, options?)` | `Promise<FileWatcher>` | Watch for changes |

## Standalone Functions

These operate on the app's built-in scope (not user-selected folders). Import from `volt:fs` in the backend or from `voltkit` in the N-API layer.

### `fs.readFile(path): Promise<string>`

Read a file as a UTF-8 string.

```ts
import { readFile } from 'volt:fs';
const content = await readFile('data/config.json');
```

### `fs.readFileBinary(path): Promise<Uint8Array>`

Read a file as raw bytes.

### `fs.writeFile(path, data): Promise<void>`

Write a UTF-8 string to a file. Creates the file and parent directories if needed.

### `fs.writeFileBinary(path, data): Promise<void>`

Write raw bytes to a file.

### `fs.readDir(path): Promise<string[]>`

List entries in a directory. Returns file and directory names.

### `fs.stat(path): Promise<FileInfo>`

Get metadata for a file or directory.

```ts
const info = await stat('data/config.json');
console.log(info.size);       // 1024
console.log(info.isFile);     // true
console.log(info.modifiedMs); // 1710345600000
console.log(info.createdMs);  // 1709827200000
```

### `fs.exists(path): Promise<boolean>`

Check whether a path exists within the scope.

```ts
if (await exists('config.json')) {
  // file is present
}
```

### `fs.mkdir(path): Promise<void>`

Create a directory. Parent directories are created automatically.

### `fs.remove(path): Promise<void>`

Remove a file or directory (recursive for directories). Refuses to remove the scope root.

### `fs.rename(from, to): Promise<void>`

Move or rename a file/directory within the scope. Both paths must be inside the scope. Fails if the destination already exists.

```ts
await rename('old-name.md', 'new-name.md');
```

### `fs.copy(from, to): Promise<void>`

Copy a file within the scope. Only files (not directories). Fails if the destination already exists.

```ts
await copy('template.md', 'notes/new-note.md');
```

## File Watching

Watch a directory for changes using the OS file notification system.

```ts
const watcher = await scopedFs.watch('', { recursive: true, debounceMs: 200 });

// Poll for events
const events = await watcher.poll();
for (const event of events) {
  console.log(event.kind, event.path); // 'create', 'notes/new-file.md'
}

// Stop watching
await watcher.close();
```

### `WatchOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `recursive` | `boolean` | `true` | Watch subdirectories |
| `debounceMs` | `number` | `200` | Debounce interval in milliseconds |

### `FileWatcher`

| Method | Returns | Description |
|--------|---------|-------------|
| `poll()` | `Promise<WatchEvent[]>` | Drain pending events |
| `close()` | `Promise<void>` | Stop watching and free resources |

### `WatchEvent`

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `'create' \| 'change' \| 'delete' \| 'rename' \| 'overflow'` | Event type |
| `path` | `string` | Relative path of the affected file |
| `oldPath` | `string?` | Previous path (for renames) |
| `isDir` | `boolean?` | Whether the path is a directory |

## Types

### `FileInfo`

```ts
interface FileInfo {
  size: number;         // File size in bytes
  isFile: boolean;      // Whether the path is a file
  isDir: boolean;       // Whether the path is a directory
  readonly: boolean;    // Whether the file is read-only
  modifiedMs: number;   // Last modification time (ms since epoch)
  createdMs?: number;   // Creation time (ms since epoch, if available)
}
```

> `createdMs` may be `undefined` on Linux filesystems that don't support birth time. Always available on Windows and macOS.

## Security

Paths are validated at two layers:

1. **TypeScript layer** — Rejects absolute paths (`/`, `\`, drive letters) and `..` traversal
2. **Rust layer** — `safe_resolve()` canonicalizes both the base and resolved paths, then verifies the result is under the base directory. The actual built-in CRUD operations then execute relative to an opened scoped directory handle, preventing symlink-swapped paths from escaping after validation. Windows reserved device names (`CON`, `NUL`, `COM1`, etc.) and symlink escapes are blocked.

Grant tokens are stored in a global `HashMap<GrantId, PathBuf>` protected by a Mutex. Grant IDs are opaque random tokens, and the stored grant root is canonicalized when the grant is created. `bindScope` validates the grant exists before creating the scoped handle — invalid or expired grant IDs are rejected.
