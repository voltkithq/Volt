# Tray

System tray icons. Requires `permissions: ['tray']`.

## `Tray`

Extends `EventEmitter`.

### Constructor

```ts
import { Tray } from 'voltkit';

const tray = new Tray({
  tooltip: 'My App',
  icon: './assets/tray-icon.png',
});
```

**Options:** `TrayOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `tooltip` | `string` | `''` | Tooltip text shown on hover |
| `icon` | `string` | — | Path to the tray icon (PNG) |
| `menu` | `TrayMenuItem[]` | - | Currently unsupported; provided items are ignored and a warning is logged |

### Methods

#### `setToolTip(tooltip: string): void`
Update the tooltip text.

#### `getToolTip(): string`
Get the current tooltip text.

#### `setImage(iconPath: string): void`
Update the tray icon.

#### `setVisible(visible: boolean): void`
Show or hide the tray icon.

#### `isVisible(): boolean`
Check if the tray icon is visible.

#### `destroy(): void`
Destroy the tray icon and clean up resources. Safe to call multiple times.

#### `isDestroyed(): boolean`
Check if the tray has been destroyed.

### Events

| Event | Description |
|-------|-------------|
| `'click'` | The tray icon was clicked |

```ts
tray.on('click', (event) => {
  mainWindow.show();
});
```

## `TrayMenuItem`

`TrayMenuItem` is reserved for future tray context-menu support. `TrayOptions.menu` is currently ignored by the runtime.

```ts
interface TrayMenuItem {
  label: string;
  enabled?: boolean;    // Default: true
  type?: 'normal' | 'separator';
  click?: () => void;
}
```

