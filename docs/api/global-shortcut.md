# Global Shortcut

Global keyboard shortcuts that work even when the app is not focused. Requires `permissions: ['globalShortcut']`.

## `globalShortcut.register(accelerator, callback): boolean`

Register a global keyboard shortcut.

```ts
import { globalShortcut } from 'voltkit';

const success = globalShortcut.register('CmdOrCtrl+Shift+P', () => {
  console.log('Shortcut triggered!');
});

if (!success) {
  console.log('Shortcut already registered');
}
```

**Parameters:**
- `accelerator: string` — Keyboard shortcut (e.g., `'CmdOrCtrl+Shift+P'`)
- `callback: () => void` — Function to call when the shortcut is pressed

**Returns:** `true` if registered, `false` if the accelerator is already registered.

## `globalShortcut.unregister(accelerator): void`

Unregister a specific global shortcut.

```ts
globalShortcut.unregister('CmdOrCtrl+Shift+P');
```

## `globalShortcut.unregisterAll(): void`

Unregister all global shortcuts. Call this on app quit to clean up.

```ts
globalShortcut.unregisterAll();
```

## `globalShortcut.isRegistered(accelerator): boolean`

Check if a shortcut is registered.

```ts
if (globalShortcut.isRegistered('CmdOrCtrl+Shift+P')) {
  console.log('Already registered');
}
```

## Accelerator Format

Accelerators use Electron-compatible syntax:

| Key | Description |
|-----|-------------|
| `CmdOrCtrl` | `Cmd` on macOS, `Ctrl` on Windows/Linux |
| `Cmd` | macOS Command key |
| `Ctrl` | Control key |
| `Alt` | Alt/Option key |
| `Shift` | Shift key |
| `A`-`Z`, `0`-`9` | Letter and number keys |
| `F1`-`F24` | Function keys |

Combine with `+`: `CmdOrCtrl+Shift+P`, `Alt+F4`, `Ctrl+Shift+I`
