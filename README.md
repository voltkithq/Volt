# Volt

Volt is a lightweight desktop application framework with a Rust runtime and a TypeScript API.
It targets familiar Electron-style ergonomics while enforcing stronger runtime security defaults.

## Status

- Pre-1.0 (`0.1.x`): APIs may still evolve.
- Actively tested across Rust and TypeScript stacks in CI.
- Runtime: Pure Rust (Boa engine) — no C/C++ FFI in the JS runtime path.

## Highlights

- Electron-style APIs (`BrowserWindow`, `ipcMain`, `Menu`, `Tray`, `dialog`, `clipboard`, `shell`)
- Rust-powered core (`wry` + `tao`) for native windowing/runtime behavior
- Pure Rust JS runtime (Boa) — memory-safe, no C FFI attack surface
- Capability-based permissions enforced in native bindings
- Signed updater path (Ed25519 + SHA-256 verification)
- Bundled frontend assets for production runtime packaging

## Build Profile

| Metric | Value |
|--------|-------|
| Clean build time | ~6–7 minutes |
| Production binary | ~21 MB |
| Debug `target/` size | ~2.5 GB (fresh), grows with incremental rebuilds |

Run `cargo clean` periodically to reclaim disk space from incremental compilation caches.

## Repository Layout

- `crates/volt-core`: Rust core runtime and platform integration
- `crates/volt-runner`: Production Boa-based JS runtime
- `crates/volt-napi`: N-API bridge from Rust to Node.js (dev mode)
- `crates/volt-updater-helper`: Self-update helper binary
- `packages/volt`: framework package published as `voltkit`
- `packages/volt-cli`: CLI toolchain
- `packages/volt-test`: E2E suite foundation
- `packages/create-volt`: project scaffolder
- `examples/`: sample apps (`hello-world`, `todo-app`, `ipc-demo`)

## Quick Start (Published Packages)

```bash
pnpm create @voltkit/create-volt my-app
cd my-app
pnpm install
pnpm dev
```

Pre-release fallback (run from this monorepo before npm packages exist):

```bash
pnpm install
pnpm --filter @voltkit/create-volt run build
node packages/create-volt/dist/index.js my-app
cd my-app
pnpm install
pnpm dev
```

Build for production:

```bash
pnpm build
pnpm package
```

## Local Development (This Monorepo)

Prerequisites:

- All platforms:
  - Node.js `>=20`
  - pnpm `>=9`
  - Rust stable toolchain
- Linux only:
  - `libwebkit2gtk-4.1-dev`
  - `libgtk-3-dev`
  - `libayatana-appindicator3-dev`
  - `librsvg2-dev`

Setup and validation:

```bash
pnpm install
pnpm build
pnpm typecheck
pnpm test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Security Notes

- Native APIs are permission-gated via `volt.config.ts`.
- IPC includes abuse bounds and payload hardening.
- Navigation and URL handling paths are scheme-restricted.
- Updater payloads are signature-verified before apply.

## Documentation

- Getting started: `docs/getting-started.md`
- 5-minute onboarding: `docs/onboarding-5-minutes.md`
- CLI reference: `docs/cli.md`
- Configuration: `docs/configuration.md`
- Security model: `docs/security.md`
- Security policy: `SECURITY.md`
- Runtime model: `docs/runtime-model.md`
- Architecture: `docs/architecture.md`
- API reference: `docs/api/README.md`
- Backend runtime modules: `docs/api/backend-runtime-modules.md`
- Hard-parts comparison: `docs/hard-parts-comparison.md`
- Testing guide: `docs/testing.md`
## License

Volt is licensed under the Business Source License 1.1. You are free to use Volt to build and ship your own applications, including commercial ones. The only restriction is that you may not offer Volt itself as a commercial product or service.

On 2030-03-07, all code released under this license converts to Apache-2.0.

See `LICENSE` for the full terms.
