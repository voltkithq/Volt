# 5-Minute Onboarding

This path gets a new Volt app running and packaged as fast as possible.

## Minute 0-1: Scaffold

```bash
pnpm create @voltkit/create-volt my-app
cd my-app
```

If you are working from this repository before package publishing, use:

```bash
pnpm install
pnpm --filter @voltkit/create-volt run build
node packages/create-volt/dist/index.js my-app
cd my-app
```

## Minute 1-2: Install + Run

```bash
pnpm install
pnpm dev
```

You should see:
- Vite dev server running
- native desktop window opened by Volt

## Minute 2-3: Add One Permission + One Native Call

Edit `volt.config.ts`:

```ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'My App',
  permissions: ['clipboard'],
});
```

In app code:

```ts
import { clipboard } from 'voltkit';

clipboard.writeText('hello from volt');
```

## Minute 3-4: Verify Packaging Prerequisites

```bash
volt doctor
```

Fix any `FAIL` checks before packaging.

## Minute 4-5: Build + Package

```bash
pnpm build
pnpm package
```

Common outputs:
- Windows: NSIS (`.exe`) and optional MSIX (`--format msix`)
- macOS: `.app`, optional `.dmg`
- Linux: `.AppImage` and `.deb`

## After 5 Minutes

1. Add typed IPC contracts for renderer/backend boundaries.
2. Add signing configuration if you need distributable installers.
3. Add smoke tests with `volt test`.
