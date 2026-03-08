import { setTimeout as delay } from 'node:timers/promises';

export interface WindowRuntimeStatus {
  windowCount: number;
  nativeReady: boolean;
  shortcut?: string;
}

export interface WaitForWindowStatusOptions {
  timeoutMs: number;
  intervalMs?: number;
  description?: string;
}

export async function waitForWindowStatus(
  readStatus: () => Promise<unknown>,
  predicate: (status: WindowRuntimeStatus) => boolean,
  options: WaitForWindowStatusOptions,
): Promise<WindowRuntimeStatus> {
  const timeoutMs = options.timeoutMs;
  const intervalMs = options.intervalMs ?? 150;
  const description = options.description ?? 'window status condition';
  const startedAt = Date.now();
  let lastStatus: WindowRuntimeStatus | null = null;

  while (Date.now() - startedAt <= timeoutMs) {
    const parsed = parseWindowStatus(readStatus ? await readStatus() : null);
    lastStatus = parsed;
    if (predicate(parsed)) {
      return parsed;
    }
    await delay(intervalMs);
  }

  const lastSnapshot = lastStatus ? JSON.stringify(lastStatus) : 'none';
  throw new Error(
    `[volt:test] timed out waiting for ${description} after ${timeoutMs}ms (last status: ${lastSnapshot}).`,
  );
}

export function assertWindowReady(status: WindowRuntimeStatus, minimumWindowCount = 1): void {
  if (!Number.isInteger(minimumWindowCount) || minimumWindowCount <= 0) {
    throw new Error('[volt:test] minimumWindowCount must be a positive integer.');
  }

  if (status.windowCount < minimumWindowCount) {
    throw new Error(
      `[volt:test] expected at least ${minimumWindowCount} open window(s), got ${status.windowCount}.`,
    );
  }

  if (!status.nativeReady) {
    throw new Error('[volt:test] expected native integrations to be ready but nativeReady=false.');
  }
}

export function parseWindowStatus(payload: unknown): WindowRuntimeStatus {
  const root = asRecord(payload);
  if (!root) {
    throw new Error('[volt:test] invalid window status payload: expected object.');
  }

  const runtime = asRecord(root.runtime);
  if (!runtime) {
    throw new Error('[volt:test] invalid window status payload: missing runtime object.');
  }

  const windowCountValue = runtime.windowCount;
  if (typeof windowCountValue !== 'number' || !Number.isInteger(windowCountValue) || windowCountValue < 0) {
    throw new Error('[volt:test] invalid window status payload: runtime.windowCount must be >= 0.');
  }
  const windowCount = windowCountValue;

  const nativeReady = runtime.nativeReady;
  if (typeof nativeReady !== 'boolean') {
    throw new Error('[volt:test] invalid window status payload: runtime.nativeReady must be boolean.');
  }

  const shortcutValue = runtime.shortcut;
  if (shortcutValue !== undefined && typeof shortcutValue !== 'string') {
    throw new Error('[volt:test] invalid window status payload: runtime.shortcut must be a string when set.');
  }
  const shortcut = typeof shortcutValue === 'string' ? shortcutValue : undefined;

  return {
    windowCount,
    nativeReady,
    shortcut,
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}
