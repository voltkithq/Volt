# BrowserWindow

Window creation and management. Electron-compatible API.

## Constructor

```ts
import { BrowserWindow } from 'voltkit';

const win = new BrowserWindow({
  width: 1024,
  height: 768,
  title: 'My Window',
});
```

**Options:** `WindowOptions` (all optional)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `width` | `number` | `800` | Window width in pixels |
| `height` | `number` | `600` | Window height in pixels |
| `title` | `string` | `'Volt'` | Window title |
| `resizable` | `boolean` | `true` | Whether the window is resizable |
| `decorations` | `boolean` | `true` | Show title bar and borders |
| `minWidth` / `minHeight` | `number` | — | Minimum dimensions |
| `maxWidth` / `maxHeight` | `number` | — | Maximum dimensions |
| `transparent` | `boolean` | `false` | Transparent background |
| `alwaysOnTop` | `boolean` | `false` | Always-on-top mode |
| `maximized` | `boolean` | `false` | Start maximized |
| `x` / `y` | `number` | — | Initial position |

## Instance Methods

### Content Loading

#### `loadURL(url: string): void`
Load a URL in the window's WebView.

- Allowed protocols: `http:`, `https:`, `volt:`
- Throws if the URL is invalid or uses any other protocol (for example `javascript:` or `data:`)

#### `loadFile(filePath: string): void`
Load a local file (creates a `file://` URL).

#### `getURL(): string | null`
Get the currently loaded URL.

### Window Properties

#### `getId(): string`
Get the unique window ID (UUID).

#### `setTitle(title: string): void` / `getTitle(): string`
Get or set the window title.

#### `setSize(width: number, height: number): void` / `getSize(): [number, number]`
Get or set the window size. Setting emits `'resize'`.

#### `setPosition(x: number, y: number): void` / `getPosition(): [number, number]`
Get or set the window position. Setting emits `'move'`.

#### `setResizable(resizable: boolean): void` / `isResizable(): boolean`
Get or set whether the window is resizable.

#### `setAlwaysOnTop(flag: boolean): void` / `isAlwaysOnTop(): boolean`
Get or set always-on-top mode.

### Window Actions

#### `maximize(): void`
Maximize the window. Emits `'maximize'`.

#### `minimize(): void`
Minimize the window. Emits `'minimize'`.

#### `restore(): void`
Restore from maximized/minimized state. Emits `'restore'`.

#### `close(): void`
Close and destroy the window.

#### `destroy(): void`
Destroy the window and free resources. Emits `'closed'`. Safe to call multiple times.

#### `isDestroyed(): boolean`
Check if the window has been destroyed. All methods except `isDestroyed()` throw after destruction.

## Static Methods

#### `BrowserWindow.getAllWindows(): BrowserWindow[]`
Get all open windows.

#### `BrowserWindow.getFocusedWindow(): BrowserWindow | null`
Get the currently focused window, or `null`.

#### `BrowserWindow.fromId(id: string): BrowserWindow | undefined`
Find a window by its ID.

## Events

| Event | Description |
|-------|-------------|
| `'closed'` | Window was destroyed |
| `'focus'` | Window gained focus |
| `'blur'` | Window lost focus |
| `'maximize'` | Window was maximized |
| `'minimize'` | Window was minimized |
| `'restore'` | Window was restored |
| `'resize'` | Window was resized |
| `'move'` | Window was moved |
