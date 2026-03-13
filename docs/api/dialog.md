# Dialog

Native file and message dialogs. Requires `permissions: ['dialog']`.

> **Important:** Dialog is a backend-only API. Call it from `backend.ts` using `volt:dialog` and expose results to the renderer via IPC.

## `showOpen(options?)`

Show a native file/folder open dialog.

```ts
import { showOpen } from 'volt:dialog';

const path = await showOpen({
  title: 'Select Image',
  filters: [{ name: 'Images', extensions: ['png', 'jpg', 'gif'] }],
});

if (path) {
  console.log('Selected:', path);
}
```

**Returns:** `Promise<string | null>` — selected path, or `null` if cancelled.

## `showOpenWithGrant(options?)`

Show a folder picker that returns a **filesystem scope grant**. This is the primary way to let users pick a folder and then perform sandboxed file operations on it.

```ts
import { showOpenWithGrant } from 'volt:dialog';
import { bindScope } from 'volt:fs';
import { ipcMain } from 'volt:ipc';

ipcMain.handle('workspace:pick', async () => {
  // 1. Open native folder picker — returns grant IDs
  const result = await showOpenWithGrant({ title: 'Open Workspace' });
  if (!result.grantIds.length) return { ok: false };

  // 2. Bind a scoped filesystem handle from the grant
  const scopedFs = await bindScope(result.grantIds[0]);

  // 3. Use the scoped handle — all paths are relative to the chosen folder
  const files = await scopedFs.readDir('');
  return { ok: true, path: result.paths[0], files };
});
```

**Returns:** `Promise<GrantDialogResult>`

| Field | Type | Description |
|-------|------|-------------|
| `paths` | `string[]` | Selected folder paths |
| `grantIds` | `string[]` | Opaque grant IDs to pass to `bindScope()` |

> **See also:** [File System — Scoped Access](./fs.md#scoped-access) for the full `ScopedFs` interface.

## `showSave(options?)`

Show a native save file dialog.

```ts
import { showSave } from 'volt:dialog';

const path = await showSave({
  title: 'Save Document',
  defaultPath: 'document.txt',
  filters: [{ name: 'Text', extensions: ['txt'] }],
});

if (path) {
  console.log('Saving to:', path);
}
```

**Returns:** `Promise<string | null>` — selected path, or `null` if cancelled.

## `showMessage(options)`

Show a native message box dialog.

```ts
import { showMessage } from 'volt:dialog';

const confirmed = await showMessage({
  dialogType: 'warning',
  title: 'Confirm',
  message: 'Are you sure you want to delete this?',
  buttons: ['Yes', 'No'],
});

if (confirmed === 1) {
  // User clicked "Yes"
}
```

**Returns:** `Promise<0 | 1>` — `1` if confirmed, `0` if cancelled/denied.

## Dialog Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string` | — | Dialog window title |
| `defaultPath` | `string` | — | Starting directory or default filename |
| `filters` | `FileFilter[]` | — | File type filters |
| `multiple` | `boolean` | `false` | Allow multiple selections |
| `directory` | `boolean` | `false` | Select directories instead of files |

## `FileFilter`

```ts
interface FileFilter {
  name: string;        // Display name (e.g., 'Images')
  extensions: string[]; // Extensions without dots (e.g., ['png', 'jpg'])
}
```

## Common Patterns

### Frontend → Backend → Dialog

Since dialog is backend-only, expose it through IPC:

```ts
// backend.ts
import { showOpenWithGrant } from 'volt:dialog';
import { ipcMain } from 'volt:ipc';

ipcMain.handle('pick-folder', async () => {
  return await showOpenWithGrant({ title: 'Choose folder' });
});
```

```ts
// renderer (main.ts)
const result = await window.__volt__.invoke('pick-folder');
```

### Dialog + Grant + Scoped FS (full flow)

See the [file-explorer example](../../examples/file-explorer/) for a complete working app that uses `showOpenWithGrant` → `bindScope` → `ScopedFs` → `watch`.
