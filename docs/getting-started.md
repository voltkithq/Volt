# Getting Started

Need a faster setup path? See [5-Minute Onboarding](onboarding-5-minutes.md).

## Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) (stable toolchain) - for native builds
- Windows only: [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (comes pre-installed on Windows 11)

## Create a Project

```bash
npm create @voltkit/volt my-app
cd my-app
```

You'll be prompted to choose a framework:
- **Vanilla TypeScript** - Minimal setup, no framework
- **React** - React + TypeScript template
- **Svelte** - Svelte + TypeScript template
- **Vue** - Vue + TypeScript template
- **Enterprise** - Vanilla TypeScript plus enterprise packaging defaults (`volt doctor`, `volt package`, ADMX/docs bundle config)

## Project Structure

```text
my-app/
|-- volt.config.ts          # Application configuration
|-- package.json
|-- vite.config.ts          # Vite build configuration (React/Svelte/Vue templates)
|-- tsconfig.json
|-- index.html
`-- src/
    |-- main.ts / main.tsx  # Frontend entry point (framework-dependent)
    |-- App.*               # Framework templates (React/Vue/Svelte)
    `-- style.css           # Template styles
```

## Development Workflow

```bash
cd my-app

# Install dependencies
npm install

# Start development (Vite dev server + native window)
npm run dev
```

This opens a native desktop window with your app loaded from the Vite dev server. Hot module replacement works automatically.

## Configuration

Edit `volt.config.ts` to configure your app:

```ts
import { defineConfig } from 'voltkit';

export default defineConfig({
  name: 'My App',
  version: '1.0.0',
  window: {
    width: 1024,
    height: 768,
    title: 'My App',
    minWidth: 400,
    minHeight: 300,
  },
  permissions: ['clipboard', 'dialog', 'fs'],
});
```

See [Configuration Reference](configuration.md) for all options.

## Building for Production

```bash
# Build the frontend + native binary
npm run build
```

The output is in `dist-volt/`. The runtime artifact includes all frontend assets embedded.

## Packaging

```bash
# Check prerequisites for packaging/signing on your machine
volt doctor

# Create platform-specific installer
npm run package
```

This produces:
- **Windows:** NSIS installer (`.exe`)
- **Windows (optional):** MSIX package (`npm run package -- --format msix`)
- **macOS:** `.app` bundle (optionally `.dmg`)
- **Linux:** AppImage and `.deb`

## Adding Capabilities

To use native APIs, declare permissions in `volt.config.ts` and import from `voltkit`:

```ts
// volt.config.ts
export default defineConfig({
  name: 'My App',
  permissions: ['clipboard', 'notification'],
});
```

```ts
// In your main process code
import { clipboard, Notification } from 'voltkit';

// Read clipboard
const text = clipboard.readText();

// Show notification
new Notification({ title: 'Hello', body: 'From Volt!' }).show();
```

## IPC Communication

Register handlers in the main process, invoke them from the renderer:

```ts
// Main process
import { ipcMain } from 'voltkit';

ipcMain.handle('get-data', async (args) => {
  return { items: ['a', 'b', 'c'], query: args.query };
});
```

```ts
// Renderer (in your frontend code)
import { invoke } from 'voltkit/renderer';

const result = await invoke('get-data', { query: 'search' });
console.log(result.items); // ['a', 'b', 'c']
```

## Next Steps

- [CLI Reference](cli.md) - All CLI commands and options
- [5-Minute Onboarding](onboarding-5-minutes.md) - Fast bootstrap and first package
- [API Reference](api/README.md) - Complete API documentation
- [Security Model](security.md) - Understanding Volt's security guarantees
- [Architecture](architecture.md) - How Volt works internally
