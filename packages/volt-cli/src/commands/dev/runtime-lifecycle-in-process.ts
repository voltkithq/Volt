import type { NativeHostWindowConfig } from '../native-host-protocol.js';
import type { NativeBinding, NativeRuntimeBridge } from './types.js';
import { extractNativeEventJson } from './runtime-event.js';

export function startInProcessRuntime(
  native: NativeBinding,
  windowConfig: NativeHostWindowConfig,
): NativeRuntimeBridge {
  const nativeApp = new native.VoltApp(windowConfig);
  nativeApp.createWindow(windowConfig);
  const primaryWindowId = windowConfig.jsId;
  let eventListener: (eventJson: string) => void = () => {};
  let resolveRunPromise: (() => void) | null = null;

  nativeApp.onEvent((...callbackArgs: unknown[]) => {
    const eventJson = extractNativeEventJson(callbackArgs);
    if (!eventJson) {
      return;
    }

    eventListener(eventJson);
    try {
      const parsed = JSON.parse(eventJson);
      if (parsed && parsed.type === 'quit') {
        resolveRunPromise?.();
      }
    } catch {
      // ignore parse errors
    }
  });

  return {
    onEvent(callback: (eventJson: string) => void): void {
      eventListener = callback;
    },
    windowEvalScript(jsId: string, script: string): void {
      native.windowEvalScript(jsId, script);
    },
    windowClose(jsId: string): void {
      native.windowClose(jsId);
    },
    windowShow(jsId: string): void {
      native.windowShow(jsId);
    },
    windowFocus(jsId: string): void {
      native.windowFocus(jsId);
    },
    windowMaximize(jsId: string): void {
      native.windowMaximize(jsId);
    },
    windowMinimize(jsId: string): void {
      native.windowMinimize(jsId);
    },
    windowRestore(jsId: string): void {
      native.windowRestore(jsId);
    },
    run(): Promise<void> {
      nativeApp.run();
      return new Promise<void>((resolve) => {
        resolveRunPromise = resolve;
      });
    },
    shutdown(): void {
      if (primaryWindowId) {
        native.windowClose(primaryWindowId);
      }
    },
  };
}
