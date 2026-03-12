# API Reference

Complete API documentation for `voltkit`.

## Modules

| Module | Description |
|--------|-------------|
| [App](app.md) | Application lifecycle (`createApp`, `getApp`, `VoltApp`) |
| [Window](window.md) | Window management (`BrowserWindow`) |
| [IPC](ipc.md) | Inter-process communication (`ipcMain`, `invoke`) |
| [Dialog](dialog.md) | Native file and message dialogs |
| [Clipboard](clipboard.md) | System clipboard read/write |
| [Data](data.md) | Native-backed data query and profiling helpers |
| [File System](fs.md) | Sandboxed file operations |
| [Menu](menu.md) | Application and context menus |
| [Notification](notification.md) | OS-level notifications |
| [Shell](shell.md) | URL opening in default browser |
| [Tray](tray.md) | System tray icons |
| [Global Shortcut](global-shortcut.md) | Global keyboard shortcuts |
| [Updater](updater.md) | Auto-update with signature verification |
| [Workflow](workflow.md) | Native-backed workflow pipeline execution |
| [Backend Runtime Modules](backend-runtime-modules.md) | Backend-only Boa modules (`volt:db`, `volt:http`, `volt:secureStorage`) |
| [Code Signing](signing.md) | macOS and Windows code signing for distribution |

## Permissions

Most modules require a permission declared in `volt.config.ts`:

| Module | Permission |
|--------|------------|
| Clipboard | `'clipboard'` |
| Notification | `'notification'` |
| Dialog | `'dialog'` |
| File System | `'fs'` |
| Database (`volt:db`) | `'db'` |
| Menu | `'menu'` |
| Shell | `'shell'` |
| HTTP (`volt:http`) | `'http'` |
| Global Shortcut | `'globalShortcut'` |
| Tray | `'tray'` |
| Secure Storage (`volt:secureStorage`) | `'secureStorage'` |

`volt:db`, `volt:http`, and `volt:secureStorage` are backend runtime modules used from `src/backend.ts` (Boa), not renderer-only APIs.

Modules without a permission requirement (App, Window, IPC, Data, Workflow, Updater) are always available.

## Importing

Most framework APIs are exported from the `voltkit` package:

```ts
import {
  createApp,
  BrowserWindow,
  ipcMain,
  dialog,
  clipboard,
  fs,
  Menu,
  MenuItem,
  Notification,
  shell,
  data,
  Tray,
  workflow,
  globalShortcut,
  autoUpdater,
  defineConfig,
} from 'voltkit';
```

Renderer-only helpers can also be imported from `voltkit/renderer`:

```ts
import { data, workflow, invoke } from 'voltkit/renderer';
```
