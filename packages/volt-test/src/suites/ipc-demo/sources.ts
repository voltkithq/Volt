export const IPC_DEMO_SMOKE_CONFIG_SOURCE = `
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

export const IPC_DEMO_FRONTEND_SMOKE_SOURCE = `
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
