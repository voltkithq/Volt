import { afterEach, beforeEach, vi } from 'vitest';
import { BrowserWindow, ipcMain } from 'voltkit';
import { __testOnly } from '../commands/dev.js';

const TEST_CHANNELS = ['sum', 'echo-load', 'hang', 'slow'] as const;

export function parseResponseScript(
  script: string,
): { id: string; result?: unknown; error?: string; errorCode?: string } {
  const match = script.match(/^window\.__volt_response__\((.+)\);$/);
  if (!match) {
    throw new Error(`Unexpected script format: ${script}`);
  }

  const responseJson = JSON.parse(match[1]) as string;
  return JSON.parse(responseJson) as {
    id: string;
    result?: unknown;
    error?: string;
    errorCode?: string;
  };
}

export function createNativeRuntimeMock() {
  return {
    windowEvalScript: vi.fn<(jsId: string, script: string) => void>(),
  };
}

export function setupDevIpcTestLifecycle(): void {
  const reset = () => {
    __testOnly.clearIpcLoadState();
    for (const win of BrowserWindow.getAllWindows()) {
      win.destroy();
    }
    for (const channel of TEST_CHANNELS) {
      if (ipcMain.hasHandler(channel)) {
        ipcMain.removeHandler(channel);
      }
    }
  };

  beforeEach(() => {
    reset();
  });

  afterEach(() => {
    vi.useRealTimers();
    reset();
  });
}
