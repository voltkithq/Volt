# Dialog

Native file and message dialogs. Requires `permissions: ['dialog']`.

## `dialog.showOpenDialog(options?)`

Show a native file open dialog.

```ts
import { dialog } from 'voltkit';

const result = await dialog.showOpenDialog({
  title: 'Select Image',
  filters: [{ name: 'Images', extensions: ['png', 'jpg', 'gif'] }],
  multiSelections: true,
});

if (!result.canceled) {
  console.log(result.filePaths);
}
```

**Options:** `OpenDialogOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string` | — | Dialog window title |
| `defaultPath` | `string` | — | Starting directory |
| `filters` | `FileFilter[]` | — | File type filters |
| `multiSelections` | `boolean` | `false` | Allow multiple selections |
| `directory` | `boolean` | `false` | Select directories instead of files |

**Returns:** `Promise<OpenDialogResult>`

| Field | Type | Description |
|-------|------|-------------|
| `canceled` | `boolean` | Whether the dialog was cancelled |
| `filePaths` | `string[]` | Selected paths (empty if cancelled) |

## `dialog.showSaveDialog(options?)`

Show a native save file dialog.

```ts
const result = await dialog.showSaveDialog({
  title: 'Save Document',
  defaultPath: 'document.txt',
  filters: [{ name: 'Text', extensions: ['txt'] }],
});

if (!result.canceled) {
  console.log(result.filePath);
}
```

**Options:** `SaveDialogOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `string` | — | Dialog window title |
| `defaultPath` | `string` | — | Default file path/name |
| `filters` | `FileFilter[]` | — | File type filters |

**Returns:** `Promise<SaveDialogResult>`

| Field | Type | Description |
|-------|------|-------------|
| `canceled` | `boolean` | Whether the dialog was cancelled |
| `filePath` | `string` | Selected path (empty string if cancelled) |

## `dialog.showMessageBox(options)`

Show a native message box dialog.

```ts
const result = await dialog.showMessageBox({
  type: 'warning',
  title: 'Confirm',
  message: 'Are you sure you want to delete this?',
  buttons: ['Yes', 'No'],
});

if (result.confirmed) {
  // proceed
}
```

**Options:** `MessageBoxOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `type` | `'info' \| 'warning' \| 'error'` | `'info'` | Dialog type |
| `title` | `string` | `''` | Dialog title |
| `message` | `string` | (required) | Message text |
| `buttons` | `string[]` | `[]` | Button labels |

**Returns:** `Promise<MessageBoxResult>`

| Field | Type | Description |
|-------|------|-------------|
| `confirmed` | `boolean` | Whether the user confirmed |

## `FileFilter`

```ts
interface FileFilter {
  name: string;        // Display name (e.g., 'Images')
  extensions: string[]; // Extensions without dots (e.g., ['png', 'jpg'])
}
```
