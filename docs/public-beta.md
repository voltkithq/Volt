# Volt Public Beta

Build desktop apps with Electron-style APIs, a Rust runtime, and security-first defaults.

## Who Beta Is For

- teams that like TypeScript ergonomics but need tighter runtime controls
- builders who want one CLI path for build, package, sign, and update
- developers shipping cross-platform desktop apps with enterprise requirements

## What You Get In Beta

- familiar desktop APIs (`BrowserWindow`, `ipcMain`, `Menu`, `Tray`, `dialog`)
- typed IPC contract path with runtime payload validation
- packaging targets for Windows/macOS/Linux, including enterprise distribution outputs
- signing and updater flows designed for production delivery
- native E2E testing foundation (`@voltkit/volt-test`)

## Public Beta Promise

1. Keep core APIs stable enough for real pilot apps.
2. Prioritize reliability and migration clarity over rapid breaking changes.
3. Publish transparent docs for known gaps and current tradeoffs.

## Start In 5 Minutes

Follow: [`docs/onboarding-5-minutes.md`](onboarding-5-minutes.md)

Quick path:

```bash
npm create @voltkit/volt my-app
cd my-app
npm install
npm run dev
```

## Before Shipping Installers

Run prerequisites check:

```bash
volt doctor
```

Then package:

```bash
npm run build
npm run package
```

## Feedback Channels

- open GitHub issues for bugs and regressions
- include platform, packaging format, and `volt doctor` output when reporting release blockers
