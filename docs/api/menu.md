# Menu

Application and context menus.

Requires the `'menu'` permission in `volt.config.ts`.

## `Menu`

### `Menu.buildFromTemplate(template)`

Build a menu from an array of item options.

```ts
import { Menu } from 'voltkit';

const menu = Menu.buildFromTemplate([
  {
    label: 'File',
    type: 'submenu',
    submenu: [
      { label: 'New', accelerator: 'CmdOrCtrl+N' },
      { type: 'separator' },
      { label: 'Quit', role: 'quit' },
    ],
  },
  {
    label: 'Edit',
    type: 'submenu',
    submenu: [
      { label: 'Copy', role: 'copy' },
      { label: 'Paste', role: 'paste' },
    ],
  },
]);
```

### `Menu.setApplicationMenu(menu)`

Set the application menu bar. Pass `null` to clear.

```ts
Menu.setApplicationMenu(menu);
```

### `Menu.getApplicationMenu(): Menu | null`

Get the current application menu.

### `menu.append(item: MenuItem): void`

Add a menu item.

### `menu.getItems(): MenuItem[]`

Get all items (returns a copy).

### `menu.toJSON(): MenuItemOptions[]`

Serialize the menu to a template for native code.

## `MenuItem`

### Constructor

```ts
import { MenuItem } from 'voltkit';

const item = new MenuItem({
  label: 'Save',
  accelerator: 'CmdOrCtrl+S',
  click: () => { /* handle click */ },
});
```

### `MenuItemOptions`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `label` | `string` | `''` | Display label |
| `accelerator` | `string` | — | Keyboard shortcut (e.g., `'CmdOrCtrl+C'`) |
| `enabled` | `boolean` | `true` | Whether the item is interactive |
| `type` | `'normal' \| 'separator' \| 'submenu'` | `'normal'` | Item type |
| `role` | `MenuItemRole` | — | Predefined system action |
| `click` | `() => void` | — | Click handler |
| `submenu` | `MenuItemOptions[]` | — | Nested items (for `type: 'submenu'`) |

### `MenuItemRole`

Predefined roles that auto-configure label and accelerator:

`'quit'` | `'copy'` | `'cut'` | `'paste'` | `'selectAll'` | `'undo'` | `'redo'` | `'minimize'` | `'separator'`
