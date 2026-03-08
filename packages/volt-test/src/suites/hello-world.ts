import { existsSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { VoltAppLauncher } from '../launcher.js';
import type { VoltTestSuite } from '../types.js';

const RESULT_FILE = '.volt-smoke-result.json';

export interface HelloWorldSmokeSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface HelloWorldSmokePayload {
  ok: boolean;
  heading: string;
  ping: { pong: number };
  durationMs: number;
}

export function createHelloWorldSmokeSuite(options: HelloWorldSmokeSuiteOptions = {}): VoltTestSuite {
  const name = options.name ?? 'hello-world-smoke';
  const projectDir = options.projectDir ?? 'examples/hello-world';
  const timeoutMs = options.timeoutMs ?? 120_000;

  return {
    name,
    timeoutMs,
    async run(context) {
      const launcher = new VoltAppLauncher({
        repoRoot: context.repoRoot,
        cliEntryPath: context.cliEntryPath,
        logger: context.logger,
      });

      await launcher.run<HelloWorldSmokePayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject: prepareHelloWorldSmokeProject,
        validatePayload: validateHelloWorldPayload,
        artifactsDir: context.artifactsDir,
      });

      await context.captureScreenshot(`${name}-post-run`);
    },
  };
}

function prepareHelloWorldSmokeProject(projectDir: string): void {
  writeFileSync(join(projectDir, 'src', 'backend.ts'), HELLO_WORLD_BACKEND_SOURCE, 'utf8');
  writeFileSync(join(projectDir, 'src', 'main.ts'), HELLO_WORLD_FRONTEND_SOURCE, 'utf8');

  const tsConfigPath = join(projectDir, 'volt.config.ts');
  if (existsSync(tsConfigPath)) {
    rmSync(tsConfigPath, { force: true });
  }
  writeFileSync(join(projectDir, 'volt.config.mjs'), HELLO_WORLD_CONFIG_SOURCE, 'utf8');
}

function validateHelloWorldPayload(payload: unknown): HelloWorldSmokePayload {
  const value = asRecord(payload);
  if (!value) {
    throw new Error('[volt:test] hello-world smoke payload must be an object.');
  }

  if (value.ok !== true) {
    throw new Error(`[volt:test] hello-world smoke failed: ${JSON.stringify(payload)}`);
  }

  const heading = value.heading;
  if (typeof heading !== 'string' || heading.trim().length === 0) {
    throw new Error('[volt:test] hello-world smoke payload missing heading text.');
  }

  const ping = asRecord(value.ping);
  if (!ping || typeof ping.pong !== 'number') {
    throw new Error('[volt:test] hello-world smoke payload missing ping.pong number.');
  }

  const durationMs = value.durationMs;
  if (typeof durationMs !== 'number' || !Number.isFinite(durationMs) || durationMs < 0) {
    throw new Error('[volt:test] hello-world smoke payload missing durationMs number.');
  }

  return {
    ok: true,
    heading,
    ping: { pong: ping.pong },
    durationMs,
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

const HELLO_WORLD_BACKEND_SOURCE = `
import { ipcMain } from 'volt:ipc';
import * as voltFs from 'volt:fs';
import * as voltWindow from 'volt:window';

const RESULT_FILE = '.volt-smoke-result.json';

ipcMain.handle('ping', () => ({ pong: Date.now() }));

ipcMain.handle('smoke:complete', async (payload) => {
  await voltFs.writeFile(RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
`.trimStart();

const HELLO_WORLD_FRONTEND_SOURCE = `
interface VoltBridge {
  invoke(method: string, args?: unknown): Promise<unknown>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

async function runSmoke(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke) {
    throw new Error('window.__volt__.invoke is unavailable');
  }

  const startedAt = Date.now();
  try {
    const heading = document.querySelector('h1')?.textContent ?? '';
    const ping = await bridge.invoke('ping');
    await bridge.invoke('smoke:complete', {
      ok: true,
      heading,
      ping,
      durationMs: Date.now() - startedAt,
    });
  } catch (error) {
    await bridge.invoke('smoke:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

void runSmoke();
`.trimStart();

const HELLO_WORLD_CONFIG_SOURCE = `
export default {
  name: 'Hello World',
  version: '0.1.0',
  backend: './src/backend.ts',
  permissions: ['fs'],
  window: {
    width: 800,
    height: 600,
    title: 'Hello Volt!',
  },
};
`.trimStart();
