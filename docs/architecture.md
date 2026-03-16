# Architecture

## Layer Diagram

```
┌──────────────────────────────────────────────────────────┐
│                    Frontend (WebView)                      │
│              HTML / CSS / JS (React, Vue, etc.)           │
│                                                           │
│  window.__volt__.invoke() ──→ IPC bridge ──→ postMessage  │
└───────────────────────┬──────────────────────────────────┘
                        │ JSON over IPC
┌───────────────────────▼──────────────────────────────────┐
│                 voltkit (TypeScript)                │
│                                                           │
│  BrowserWindow  ipcMain  Menu   Tray   dialog  clipboard │
│  fs  shell  Notification  globalShortcut  autoUpdater    │
│                                                           │
│  Validates inputs, maps camelCase → snake_case, provides │
│  Electron-compatible API surface                          │
└───────────────────────┬──────────────────────────────────┘
                        │ N-API function calls
┌───────────────────────▼──────────────────────────────────┐
│                   volt-napi (Rust → Node.js)              │
│                                                           │
│  #[napi] functions and classes that bridge TypeScript     │
│  calls to volt-core. Handles JSON ↔ Rust type conversion │
└───────────────────────┬──────────────────────────────────┘
                        │ Rust function calls
┌───────────────────────▼──────────────────────────────────┐
│                   volt-core (Rust library)                 │
│                                                           │
│  16 modules implementing all native functionality:        │
│  app · window · webview · ipc · security · fs · embed    │
│  menu · tray · clipboard · dialog · notification         │
│  global_shortcut · shell · updater · permissions          │
└───────────────────────┬──────────────────────────────────┘
                        │ System calls
┌───────────────────────▼──────────────────────────────────┐
│                   System Libraries                        │
│  wry (WebView)  tao (windowing)  arboard (clipboard)     │
│  rfd (dialogs)  notify-rust  muda (menus)  tray-icon    │
│  ed25519-dalek  reqwest  semver                          │
└──────────────────────────────────────────────────────────┘
```

## Module Map

### volt-core (Rust)

| Module | Purpose |
|--------|---------|
| `app` | Application lifecycle, event loop, `AppConfig`, `AppEvent` |
| `window` | `WindowConfig` with serde defaults, `WindowHandle` |
| `webview` | Navigation policy, origin whitelisting, `WebViewConfig` |
| `ipc` | `IpcRegistry`, `IpcRequest`/`IpcResponse`, `RateLimiter`, prototype pollution guards |
| `security` | CSP generation (prod/dev), path validation, URL scheme validation, reserved device name blocking |
| `fs` | `safe_resolve()` with canonicalization plus capability-scoped sandboxed CRUD operations |
| `embed` | `AssetBundle` (serialize/deserialize/serve), MIME type detection, `volt://` protocol handler |
| `menu` | `MenuItemConfig`, role-based predefined items, `build_menu()` |
| `tray` | `TrayConfig` for system tray icons |
| `clipboard` | Read/write text and images with size limits |
| `dialog` | `OpenDialogOptions`, `SaveDialogOptions`, `MessageDialogOptions` |
| `notification` | `NotificationConfig` for OS notifications |
| `global_shortcut` | `ShortcutManager` tracking registered accelerators |
| `shell` | `open_external()` with protocol allow-list |
| `updater` | `UpdateConfig`, Ed25519 verification, SHA-256 hashing, semver downgrade check |
| `permissions` | `Permission` enum, `CapabilityGuard` for runtime checks |

### voltkit (TypeScript)

| Module | Exports |
|--------|---------|
| `app` | `VoltApp`, `createApp()`, `getApp()`, `resetApp()` |
| `window` | `BrowserWindow` (constructor, instance/static methods, events) |
| `ipc` | `ipcMain` (handle/removeHandler/processRequest), `invoke()`, `on()`, `off()` |
| `fs` | `fs.readFile()`, `writeFile()`, `readDir()`, `stat()`, `mkdir()`, `remove()` |
| `shell` | `shell.openExternal()` |
| `clipboard` | `clipboard.readText()`, `writeText()`, `readImage()`, `writeImage()` |
| `dialog` | `dialog.showOpenDialog()`, `showSaveDialog()`, `showMessageBox()` |
| `menu` | `Menu`, `MenuItem` |
| `notification` | `Notification` |
| `tray` | `Tray` |
| `globalShortcut` | `globalShortcut.register()`, `unregister()`, `unregisterAll()`, `isRegistered()` |
| `updater` | `autoUpdater` (checkForUpdates, downloadUpdate, quitAndInstall) |
| `types` | `VoltConfig`, `WindowOptions`, `Permission`, `defineConfig()` |

## IPC Flow

```
Renderer (WebView)                     Main Process (Node.js)
─────────────────                      ──────────────────────
invoke('get-user', {id: 1})
    │
    ▼
window.__volt__.invoke()
    │
    ▼  JSON.stringify({id, method, args})
window.ipc.postMessage(json)
    │
    ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌▶ volt-napi receives message
                                            │
                                            ▼  Rate limit check
                                            ▼  Prototype pollution check
                                            ▼  Parse IpcRequest
                                            ▼  Look up handler
                                       ipcMain.handle('get-user', fn)
                                            │
                                            ▼  Execute handler
                                       IpcResponse { id, result }
                                            │
    ◀╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ evaluate_script(response_script)
    │
    ▼
window.__volt_response__(json)
    │
    ▼  Resolve pending Promise
callback receives { name: 'Alice' }
```

## Asset Bundling Pipeline

```
Frontend Source (src/)
    │
    ▼  vite build
Build Output (dist/)
    ├── index.html
    ├── assets/main-abc123.js
    └── assets/style-def456.css
    │
    ▼  createAssetBundle()
Binary Bundle (.volt-assets.bin)
    Format: count(u32le) + [path_len + path + data_len + data] × count
    │
    ▼  include_bytes! (Rust compilation)
Embedded in Binary
    │
    ▼  AssetBundle::from_bytes()
In-Memory Asset Map
    │
    ▼  serve_asset() via volt:// protocol
HTTP Response with CSP headers + correct MIME type
```

## Permission Enforcement Flow

```

## Observability Hooks

- Command trace IDs:
  - Every NAPI-to-event-loop command now carries a generated `trace_id` in the command envelope.
- Command timing:
  - The event loop records queue delay and processing duration per command and logs slow commands.
- Command counters:
  - `volt-core::command` tracks sent/processed/failed command counts and exposes a snapshot API.
- Dropped callback counter:
  - `volt-napi` now counts non-blocking callback dispatch failures and prints a shutdown summary when non-zero.
volt.config.ts                    Runtime
──────────────                    ───────
permissions: ['clipboard', 'fs']
    │
    ▼  loadConfig()
CapabilityGuard { granted: {Clipboard, FileSystem} }
    │
    ▼  API call: clipboard.readText()
    ▼  guard.check(Permission::Clipboard) → Ok
    ▼  Execute native operation
    ▼  Return result

    ▼  API call: shell.openExternal(...)
    ▼  guard.check(Permission::Shell) → Err(UndeclaredCapability)
    ▼  Error: "undeclared capability: 'shell' is not listed..."
```
