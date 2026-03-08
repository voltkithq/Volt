import { preview as vitePreview } from 'vite';
import { randomUUID } from 'node:crypto';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { loadConfig } from '../utils/config.js';
import type { NativeHostWindowConfig } from './native-host-protocol.js';

interface NativeBinding {
  VoltApp: new (config: NativeHostWindowConfig) => {
    createWindow(config: NativeHostWindowConfig): void;
    onEvent(callback: (event: string) => void): void;
    run(): void;
  };
}

function loadNativeBinding(): NativeBinding | null {
  try {
    const require = createRequire(import.meta.url);
    return require('@voltkit/volt-native') as NativeBinding;
  } catch {
    return null;
  }
}

/**
 * Preview the production build locally using Vite's preview server.
 */
export async function previewCommand(): Promise<void> {
  const cwd = process.cwd();
  console.log('[volt] Starting preview server...');

  const config = await loadConfig(cwd);
  const outDir = config.build?.outDir ?? 'dist';

  try {
    const server = await vitePreview({
      root: cwd,
      build: {
        outDir: resolve(cwd, outDir),
      },
    });

    const address = server.resolvedUrls?.local?.[0] ?? 'http://localhost:4173';
    console.log(`[volt] Preview server running at ${address}`);

    const native = loadNativeBinding();
    if (!native) {
      console.log('[volt] Native binding not available. Open the preview URL in your browser.');
      console.log('[volt] Press Ctrl+C to stop.');
      return;
    }

    console.log('[volt] Opening native preview window...');
    const windowConfig: NativeHostWindowConfig = {
      name: config.name,
      permissions: config.permissions ?? [],
      jsId: randomUUID(),
      url: address,
      devtools: false,
      window: {
        title: config.window?.title ?? config.name,
        width: config.window?.width ?? 800,
        height: config.window?.height ?? 600,
        minWidth: config.window?.minWidth,
        minHeight: config.window?.minHeight,
        resizable: config.window?.resizable ?? true,
        decorations: config.window?.decorations ?? true,
      },
    };
    const app = new native.VoltApp(windowConfig);
    app.createWindow(windowConfig);
    app.onEvent((eventJson: string) => {
      try {
        const event = JSON.parse(eventJson);
        if (event?.type === 'quit') {
          void server.close();
        }
      } catch {
        // ignore malformed event payloads from native runtime
      }
    });
    app.run();
    await server.close();
  } catch (err) {
    console.error('[volt] Preview failed:', err);
    process.exit(1);
  }
}
