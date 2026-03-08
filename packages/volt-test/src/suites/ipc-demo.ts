import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { MenuAutomationDriver, TrayAutomationDriver, type AutomationEvent } from '../drivers/index.js';
import { VoltAppLauncher } from '../launcher.js';
import { assertWindowReady, parseWindowStatus } from '../window.js';
import type { VoltTestSuite } from '../types.js';

const RESULT_FILE = '.volt-smoke-result.json';

export interface IpcDemoSmokeSuiteOptions {
  name?: string;
  projectDir?: string;
  timeoutMs?: number;
}

export interface IpcDemoSmokePayload {
  ok: boolean;
  ping: { pong: number };
  echo: { message: string; sentAt: string };
  compute: { sum: number; product: number };
  nativeSetup: unknown;
  status: unknown;
  dbList: unknown;
  events: AutomationEvent[];
}

export function createIpcDemoSmokeSuite(options: IpcDemoSmokeSuiteOptions = {}): VoltTestSuite {
  const name = options.name ?? 'ipc-demo-smoke';
  const projectDir = options.projectDir ?? 'examples/ipc-demo';
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
      const menuDriver = new MenuAutomationDriver();
      const trayDriver = new TrayAutomationDriver();

      const payload = await launcher.run<IpcDemoSmokePayload>({
        sourceProjectDir: projectDir,
        resultFile: RESULT_FILE,
        timeoutMs: context.timeoutMs,
        prepareProject: prepareIpcDemoSmokeProject,
        validatePayload: validateIpcDemoPayload,
        artifactsDir: context.artifactsDir,
      });

      const menuSetup = menuDriver.parseSetupPayload(payload.nativeSetup);
      const traySetup = trayDriver.parseSetupPayload(payload.nativeSetup);
      const windowStatus = parseWindowStatus(payload.status);

      if (!menuSetup.shortcutRegistered) {
        context.logger.warn('[volt:test] ipc-demo shortcut registration failed (accepted in headless CI).');
      }

      if (!traySetup.trayReady) {
        context.logger.warn('[volt:test] ipc-demo tray setup reported trayReady=false (accepted in headless CI).');
      }

      const menuClicks = menuDriver.countClickEvents(payload.events);
      const trayClicks = trayDriver.countClickEvents(payload.events);
      assertWindowReady(windowStatus, 1);
      context.logger.log(
        `[volt:test] ipc-demo event summary: menuClicks=${menuClicks}, trayClicks=${trayClicks}, totalEvents=${payload.events.length}`,
      );
      await context.captureScreenshot(`${name}-post-run`);
    },
  };
}

function prepareIpcDemoSmokeProject(projectDir: string): void {
  const backendPath = join(projectDir, 'src', 'backend.ts');
  const backendSource = readFileSync(backendPath, 'utf8');
  writeFileSync(backendPath, patchBackendForSmoke(backendSource), 'utf8');

  const tsConfigPath = join(projectDir, 'volt.config.ts');
  rmSync(tsConfigPath, { force: true });
  writeFileSync(join(projectDir, 'volt.config.mjs'), IPC_DEMO_SMOKE_CONFIG_SOURCE, 'utf8');

  const mainPath = join(projectDir, 'src', 'main.ts');
  writeFileSync(mainPath, IPC_DEMO_FRONTEND_SMOKE_SOURCE, 'utf8');
}

function validateIpcDemoPayload(payload: unknown): IpcDemoSmokePayload {
  const value = asRecord(payload);
  if (!value) {
    throw new Error('[volt:test] ipc-demo smoke payload must be an object.');
  }

  if (value.ok !== true) {
    throw new Error(`[volt:test] ipc-demo smoke failed: ${JSON.stringify(payload)}`);
  }

  const ping = asRecord(value.ping);
  if (!ping || typeof ping.pong !== 'number') {
    throw new Error('[volt:test] ipc-demo smoke payload missing ping.pong number.');
  }

  const echo = asRecord(value.echo);
  if (!echo || typeof echo.message !== 'string' || typeof echo.sentAt !== 'string') {
    throw new Error('[volt:test] ipc-demo smoke payload missing echo object.');
  }

  const compute = asRecord(value.compute);
  const computeSum = compute?.sum;
  const computeProduct = compute?.product;
  if (
    !compute
    || typeof computeSum !== 'number'
    || typeof computeProduct !== 'number'
    || computeSum !== 23
    || computeProduct !== 42
  ) {
    throw new Error('[volt:test] ipc-demo smoke payload has invalid compute result.');
  }

  const events = normalizeEvents(value.events);
  if (events.length === 0) {
    throw new Error('[volt:test] ipc-demo smoke payload must include at least one native event.');
  }

  const dbList = value.dbList;
  if (!Array.isArray(dbList) || dbList.length === 0) {
    throw new Error('[volt:test] ipc-demo smoke payload expected non-empty dbList.');
  }

  return {
    ok: true,
    ping: { pong: ping.pong },
    echo: {
      message: echo.message,
      sentAt: echo.sentAt,
    },
    compute: {
      sum: computeSum,
      product: computeProduct,
    },
    nativeSetup: value.nativeSetup,
    status: value.status,
    dbList,
    events,
  };
}

function normalizeEvents(value: unknown): AutomationEvent[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((entry) => {
      const record = asRecord(entry);
      if (!record || typeof record.event !== 'string') {
        return null;
      }
      return {
        event: record.event,
        payload: record.payload,
      } satisfies AutomationEvent;
    })
    .filter((entry): entry is AutomationEvent => entry !== null);
}

function patchBackendForSmoke(source: string): string {
  let next = source;
  const fsImport = "import * as voltFs from 'volt:fs';";
  if (!next.includes(fsImport)) {
    const anchorImport = "import * as voltEvents from 'volt:events';";
    if (!next.includes(anchorImport)) {
      throw new Error('[volt:test] failed to patch ipc-demo backend: expected voltEvents import anchor.');
    }
    next = next.replace(anchorImport, `${anchorImport}\n${fsImport}`);
  }

  if (next.includes("ipcMain.handle('smoke:complete'")) {
    return next;
  }

  return `${next.trimEnd()}

const IPC_DEMO_SMOKE_RESULT_FILE = '.volt-smoke-result.json';

ipcMain.handle('smoke:complete', async (payload: unknown) => {
  await voltFs.writeFile(IPC_DEMO_SMOKE_RESULT_FILE, JSON.stringify(payload));
  setTimeout(() => {
    voltWindow.quit();
  }, 50);
  return { ok: true };
});
`;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

const IPC_DEMO_SMOKE_CONFIG_SOURCE = `
export default {
  name: 'IPC Demo',
  version: '0.1.0',
  backend: './src/backend.ts',
  window: {
    width: 980,
    height: 760,
    title: 'Volt IPC Demo',
    minWidth: 860,
    minHeight: 640,
  },
  permissions: ['clipboard', 'db', 'menu', 'globalShortcut', 'tray', 'secureStorage', 'fs'],
};
`.trimStart();

const IPC_DEMO_FRONTEND_SMOKE_SOURCE = `
interface VoltBridge {
  invoke(method: string, args?: unknown): Promise<unknown>;
  on(event: string, callback: (payload: unknown) => void): void;
  off(event: string, callback: (payload: unknown) => void): void;
}

interface AutomationEvent {
  event: string;
  payload: unknown;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

const events: AutomationEvent[] = [];

function appendEvent(event: string, payload: unknown): void {
  events.push({ event, payload });
}

function bindEvent(bridge: VoltBridge, event: string): void {
  bridge.on(event, (payload) => {
    appendEvent(event, payload);
  });
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}

function isWindowReadyStatus(value: unknown): boolean {
  const root = asRecord(value);
  const runtime = asRecord(root?.runtime);
  const windowCount = runtime?.windowCount;
  return typeof windowCount === 'number' && windowCount >= 1;
}

async function waitForWindowReady(bridge: VoltBridge, timeoutMs: number): Promise<unknown> {
  const startedAt = Date.now();
  let lastStatus: unknown = null;

  while (Date.now() - startedAt <= timeoutMs) {
    lastStatus = await bridge.invoke('status');
    if (isWindowReadyStatus(lastStatus)) {
      return lastStatus;
    }
    await sleep(120);
  }

  throw new Error(
    'timed out waiting for native-ready window status: '
      + JSON.stringify(lastStatus),
  );
}

async function runSmoke(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke || !bridge.on) {
    throw new Error('window.__volt__ bridge is unavailable');
  }

  bindEvent(bridge, 'demo:menu-click');
  bindEvent(bridge, 'demo:tray-click');
  bindEvent(bridge, 'demo:native-ready');
  bindEvent(bridge, 'demo:native-error');
  bindEvent(bridge, 'demo:db-updated');

  try {
    const readyStatus = await waitForWindowReady(bridge, 12_000);
    const ping = await bridge.invoke('ping');
    const echoPayload = {
      message: 'smoke-echo',
      sentAt: new Date().toISOString(),
    };
    const echo = await bridge.invoke('echo', echoPayload);
    const compute = await bridge.invoke('compute', { a: 21, b: 2 });
    const nativeSetup = await bridge.invoke('native:setup');
    await bridge.invoke('db:add', { message: 'smoke-row' });
    const dbList = await bridge.invoke('db:list');
    const status = await bridge.invoke('status');

    await bridge.invoke('smoke:complete', {
      ok: true,
      ping,
      echo,
      compute,
      nativeSetup,
      status: status ?? readyStatus,
      dbList,
      events,
    });
  } catch (error) {
    await bridge.invoke('smoke:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
      events,
    });
  }
}

void runSmoke();
`.trimStart();
