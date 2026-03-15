import type { AutomationEvent } from '../../drivers/index.js';

import type { IpcDemoSmokePayload } from './types.js';

export function validateIpcDemoPayload(payload: unknown): IpcDemoSmokePayload {
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
    !compute ||
    typeof computeSum !== 'number' ||
    typeof computeProduct !== 'number' ||
    computeSum !== 23 ||
    computeProduct !== 42
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

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object') {
    return null;
  }
  return value as Record<string, unknown>;
}
