interface VoltBridge {
  invoke(method: string, args?: unknown): Promise<unknown>;
}

declare global {
  interface Window {
    __volt__?: VoltBridge;
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}

async function waitForWindowReady(bridge: VoltBridge, timeoutMs: number): Promise<unknown> {
  const startedAt = Date.now();
  let lastStatus: unknown = null;

  while (Date.now() - startedAt <= timeoutMs) {
    lastStatus = await bridge.invoke('e2e:status');
    const runtime = asRecord(asRecord(lastStatus)?.runtime);
    if (typeof runtime?.windowCount === 'number' && runtime.windowCount >= 1 && runtime.nativeReady === true) {
      return lastStatus;
    }

    await new Promise((resolve) => {
      setTimeout(resolve, 120);
    });
  }

  throw new Error(`Timed out waiting for window-ready status: ${JSON.stringify(lastStatus)}`);
}

async function runE2e(): Promise<void> {
  const bridge = window.__volt__;
  if (!bridge?.invoke) {
    throw new Error('window.__volt__.invoke is unavailable');
  }

  try {
    const status = await waitForWindowReady(bridge, 12_000);
    const openDialogResult = await bridge.invoke('e2e:dialog:open');
    await bridge.invoke('e2e:complete', {
      ok: true,
      status,
      openDialogResult,
    });
  } catch (error) {
    await bridge.invoke('e2e:complete', {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

void runE2e();
