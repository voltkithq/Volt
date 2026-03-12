<p align="center">
  <img src="assets/volt-logo.png" alt="Volt" width="128" height="128">
</p>

<h1 align="center">Volt</h1>

[![CI](https://github.com/voltkithq/Volt/actions/workflows/ci.yml/badge.svg)](https://github.com/voltkithq/Volt/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/voltkit)](https://www.npmjs.com/package/voltkit)
[![License: BSL 1.1](https://img.shields.io/badge/license-BSL--1.1-blue)](LICENSE)

Build desktop apps with TypeScript and web technologies, powered by a Rust runtime.

Volt is a TypeScript-first desktop framework for teams that want a smaller, safer, more native stack than Electron without forcing app developers to write Rust. Main-process TypeScript handles orchestration, windows, plugins, and background workflows, while performance-sensitive work runs through Rust-backed Volt APIs.

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
| Backend language | JavaScript | Rust | TypeScript orchestration + Rust-backed APIs |
| Learning curve | Low (all JS) | Steep (must write Rust) | Low (all TypeScript) |
| Permission model | None by default | Capability config | Capability config |
| API style | `require('electron')` | Rust commands | `ipcMain`, `BrowserWindow`, `data`, `workflow` |

Volt sits between Electron and Tauri. You get Electron-style TypeScript app authoring with a Rust-backed runtime, capability-based permissions by default, and native fast paths for heavy operations. Volt is not trying to be a drop-in Electron replacement for arbitrary main-process JavaScript workloads.

## What Volt Is

- **TypeScript-first for app developers** -- frontend and backend code stay in TypeScript
- **Rust-powered under the hood** -- heavy work can run through native Volt APIs without exposing Rust to app authors
- **Orchestration-first in the main process** -- Boa runs lifecycle, IPC coordination, plugins, and background workflows
- **Native-backed where it matters** -- use `data` and `workflow` APIs when work should not stay in interpreted JS

## What Volt Is Not

- **Not a full Electron compatibility layer** -- Volt keeps familiar patterns, but it does not aim to run every Electron/Node desktop package unchanged
- **Not a “write Rust to do anything serious” framework** -- that is the tradeoff many teams want to avoid
- **Not a general high-performance JS compute runtime** -- heavy main-process operations should use Volt's Rust-backed APIs

## What's Included

- **Familiar APIs** -- `ipcMain.handle()`, `BrowserWindow`, `Menu`, `Tray`, `globalShortcut`, `dialog`, `clipboard`, `shell`, `nativeTheme`
- **Capability-based permissions** -- declare what your app can access in `volt.config.ts`, everything else is denied
- **Native-backed fast paths** -- `data` and `workflow` APIs for heavy queries and pipeline execution without writing Rust
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

A Volt app has a frontend (any web framework) and a backend (`src/backend.ts`) that handles orchestration and IPC:

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
import { ipcMain } from 'voltkit';

ipcMain.handle('get-users', async () => {
  return [
    { id: 1, name: 'Ada', role: 'admin' },
    { id: 2, name: 'Linus', role: 'member' },
  ];
});
```

Example frontend:

```typescript
import { data, invoke, workflow } from 'voltkit/renderer';

const users = await invoke('get-users');
const profile = await data.profile({ datasetSize: 12_000 });
const result = await workflow.run({ batchSize: 3_000, passes: 3 });
```

## Status

Pre-1.0 release (`0.1.x`). Core APIs are stable enough to build against, but Volt is still defining its exact product boundaries. The current direction is clear: TypeScript-first app authoring, Rust-backed performance for heavy paths, and capability-based desktop APIs with a smaller footprint than Electron.

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
