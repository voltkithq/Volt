import {
  clearRuntimeBindings,
  replaceRuntimeBindings,
  type RuntimeBinding,
} from '../runtime-event-bindings.js';
import { ensureBridge } from './bridge.js';
import type { DomRefs } from './dom.js';

declare global {
  interface Window {
    __voltIpcDemoRuntimeBindings__?: RuntimeBinding[];
  }
}

interface RuntimeEventCallbacks {
  handleDbList(): Promise<void>;
  handleStatus(): Promise<void>;
  handleSecretHas(): Promise<void>;
}

const eventLog: string[] = [];

function appendEventLog(eventsResult: HTMLPreElement, eventName: string, payload: unknown): void {
  const line = `[${new Date().toLocaleTimeString()}] ${eventName}: ${JSON.stringify(payload)}`;
  eventLog.unshift(line);
  if (eventLog.length > 14) {
    eventLog.pop();
  }
  eventsResult.textContent = eventLog.join('\n');
}

export function bindRuntimeEvents(
  dom: Pick<DomRefs, 'eventsResult' | 'progressResult' | 'nativeValue'>,
  callbacks: RuntimeEventCallbacks,
): void {
  const bridge = ensureBridge();
  const bindings: RuntimeBinding[] = [
    {
      event: 'demo:progress',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:progress', payload);
        const percent = (payload as { percent?: unknown })?.percent;
        if (typeof percent === 'number') {
          dom.progressResult.textContent = `Progress: ${percent}%`;
        }
      },
    },
    {
      event: 'demo:menu-click',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:menu-click', payload);
      },
    },
    {
      event: 'demo:shortcut',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:shortcut', payload);
        dom.nativeValue.textContent = 'shortcut triggered';
      },
    },
    {
      event: 'demo:tray-click',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:tray-click', payload);
      },
    },
    {
      event: 'demo:db-updated',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:db-updated', payload);
        void callbacks.handleDbList();
        void callbacks.handleStatus();
      },
    },
    {
      event: 'demo:native-ready',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:native-ready', payload);
        dom.nativeValue.textContent = 'ready';
      },
    },
    {
      event: 'demo:native-error',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:native-error', payload);
        dom.nativeValue.textContent = 'error';
      },
    },
    {
      event: 'demo:secure-storage-updated',
      callback: (payload) => {
        appendEventLog(dom.eventsResult, 'demo:secure-storage-updated', payload);
        void callbacks.handleSecretHas();
        void callbacks.handleStatus();
      },
    },
  ];

  window.__voltIpcDemoRuntimeBindings__ = replaceRuntimeBindings(
    bridge,
    window.__voltIpcDemoRuntimeBindings__,
    bindings,
  );
}

export function unbindRuntimeEvents(): void {
  const bridge = window.__volt__;
  if (!bridge) {
    window.__voltIpcDemoRuntimeBindings__ = [];
    return;
  }

  window.__voltIpcDemoRuntimeBindings__ = clearRuntimeBindings(
    bridge,
    window.__voltIpcDemoRuntimeBindings__,
  );
}
