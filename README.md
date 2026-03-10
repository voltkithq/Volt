# Volt

[![CI](https://github.com/voltkithq/Volt/actions/workflows/ci.yml/badge.svg)](https://github.com/voltkithq/Volt/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/voltkit)](https://www.npmjs.com/package/voltkit)
[![License: BSL 1.1](https://img.shields.io/badge/license-BSL--1.1-blue)](LICENSE)

Build desktop apps with TypeScript and web technologies, powered by a Rust runtime.

If you know Electron, you already know Volt. Same API patterns (`ipcMain`, `BrowserWindow`, `Menu`, `Tray`, `dialog`, `clipboard`, `shell`) but with a Rust core instead of Chromium, capability-based permissions by default, and ~21 MB production binaries.

## Quick Start

```bash
npx @voltkit/create-volt my-app
cd my-app
npm install
npm run dev
```

Supports Vanilla, React, Vue, Svelte, and Enterprise templates out of the box. Pass `--framework react` (or `vue`, `svelte`, `enterprise`) to skip the prompt.

Build and package for distribution:

```bash
npm run build
npm run package
```

## Why Volt over Electron or Tauri?

| | Electron | Tauri | Volt |
|---|---|---|---|
| Runtime | Chromium + Node.js | Rust + system webview | Rust + system webview |
| Binary size | ~150 MB+ | ~3-10 MB | ~21 MB |
| Backend language | JavaScript | Rust | TypeScript (runs on a Rust JS engine) |
| Learning curve | Low (all JS) | Steep (must write Rust) | Low (all TypeScript) |
| Permission model | None by default | Capability config | Capability config |
| API style | `require('electron')` | Rust commands | `import { ipcMain } from 'volt:ipc'` |

Volt sits between Electron and Tauri. You get the small binary and native performance of a Rust-backed runtime without having to write any Rust yourself. Your backend code is TypeScript, running on Boa (a pure-Rust JS engine) in production.

## What's Included

- **Familiar APIs** -- `ipcMain.handle()`, `BrowserWindow`, `Menu`, `Tray`, `globalShortcut`, `dialog`, `clipboard`, `shell`, `nativeTheme`
- **Capability-based permissions** -- declare what your app can access in `volt.config.ts`, everything else is denied
- **Built-in updater** -- Ed25519 signed updates with SHA-256 verification, no third-party services required
- **Embedded SQLite** -- `volt:db` module for local storage without external dependencies
- **Secure file access** -- scoped `volt:fs` module with path restrictions
- **Dev experience** -- Vite-powered HMR in dev, single binary output in production
- **Cross-platform** -- Windows, macOS (Intel + Apple Silicon), Linux

## Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) stable toolchain
- Windows: WebView2 Runtime (pre-installed on Windows 10/11)
- Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`

## Project Structure

A Volt app has a frontend (any web framework) and a backend (`src/backend.ts`) that handles IPC:

```
my-app/
  src/
    main.ts          # frontend entry
    backend.ts       # backend IPC handlers
  volt.config.ts     # app config + permissions
  vite.config.ts     # Vite config
  package.json
```

Example backend:

```typescript
import { ipcMain } from 'volt:ipc';
import * as db from 'volt:db';

ipcMain.handle('get-users', async () => {
  return db.query('SELECT * FROM users');
});
```

Example frontend:

```typescript
const users = await window.__volt__.invoke('get-users');
```

## Status

Pre-1.0 release (`0.1.x`). Core APIs are stable but may still evolve. Actively tested across Windows, macOS, and Linux in CI.

## Documentation

- [Getting Started](docs/getting-started.md)
- [5-Minute Onboarding](docs/onboarding-5-minutes.md)
- [Configuration](docs/configuration.md)
- [CLI Reference](docs/cli.md)
- [API Reference](docs/api/README.md)
- [Security Model](docs/security.md)
- [Architecture](docs/architecture.md)
- [Framework Comparison](docs/hard-parts-comparison.md)

## Contributing

```bash
pnpm install
pnpm build
cargo test --workspace
pnpm test
```

See the [architecture docs](docs/architecture.md) for an overview of the codebase.

## License

Business Source License 1.1. You can use Volt to build and ship any application, including commercial ones. The only restriction is you cannot offer Volt itself as a competing product or service.

All code converts to Apache-2.0 on 2030-03-07.

See [LICENSE](LICENSE) for full terms.
