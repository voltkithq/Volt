import { describe, expect, it, vi } from 'vitest';
import { BrowserWindow, ipcMain } from 'voltkit';
import { __testOnly } from '../commands/dev.js';
import {
  createNativeRuntimeMock,
  parseResponseScript,
  setupDevIpcTestLifecycle,
} from './dev-ipc.shared.js';

describe('dev IPC round-trip: in-flight limits and runtime reset', () => {
  setupDevIpcTestLifecycle();

  it('applies per-window in-flight limits and rejects overflow deterministically', async () => {
    const pendingResolves: Array<(value: unknown) => void> = [];
    ipcMain.handle(
      'slow',
      () =>
        new Promise((resolve) => {
          pendingResolves.push(resolve);
        }),
    );

    const native = createNativeRuntimeMock();
    const requests = Array.from({ length: 10 }, (_, index) =>
      __testOnly.handleIpcMessageEvent(
        native,
        {
          type: 'ipc-message',
          windowId: 'window-capped',
          raw: { id: `req-${index}`, method: 'slow', args: { index } },
        },
        { maxInFlightPerWindow: 2, timeoutMs: 500 },
      ),
    );

    await Promise.resolve();
    expect(native.windowEvalScript).toHaveBeenCalledTimes(8);
    const immediateResponses = native.windowEvalScript.mock.calls.map(([, script]) =>
      parseResponseScript(script),
    );
    const overflowResponses = immediateResponses.filter(
      (response) => response.errorCode === 'IPC_IN_FLIGHT_LIMIT',
    );
    expect(overflowResponses).toHaveLength(8);

    for (const resolve of pendingResolves) {
      resolve({ ok: true });
    }
    await Promise.all(requests);

    expect(native.windowEvalScript).toHaveBeenCalledTimes(10);
    const finalResponses = native.windowEvalScript.mock.calls.map(([, script]) =>
      parseResponseScript(script),
    );
    const successResponses = finalResponses.filter((response) => response.result != null);
    expect(successResponses).toHaveLength(2);
  });

  it('clears per-window in-flight state when the window closes', async () => {
    vi.useFakeTimers();

    ipcMain.handle('hang', async () => {
      await new Promise(() => {});
      return { unreachable: true };
    });
    ipcMain.handle('sum', (args) => args);

    const native = createNativeRuntimeMock();
    const staleRequest = __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-restart',
        raw: { id: 'req-stale', method: 'hang', args: null },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );

    await Promise.resolve();

    __testOnly.handleWindowClosedEventForIpcState({
      type: 'window-closed',
      windowId: 'WindowId(42)',
      jsWindowId: 'window-restart',
    });

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-restart',
        raw: { id: 'req-fresh', method: 'sum', args: { reopened: true } },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );

    await vi.advanceTimersByTimeAsync(55);
    await staleRequest;

    const responses = native.windowEvalScript.mock.calls.map(([, script]) => parseResponseScript(script));
    expect(responses.find((response) => response.id === 'req-fresh')).toEqual({
      id: 'req-fresh',
      result: { reopened: true },
    });
    expect(responses.filter((response) => response.errorCode === 'IPC_IN_FLIGHT_LIMIT')).toHaveLength(0);
  });

  it('clears all in-flight slots when window-closed omits jsWindowId', async () => {
    vi.useFakeTimers();

    ipcMain.handle('hang', async () => {
      await new Promise(() => {});
      return null;
    });
    ipcMain.handle('sum', (args) => args);

    const native = createNativeRuntimeMock();
    const staleRequest = __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-without-js-close-id',
        raw: { id: 'req-stale-null-js-id', method: 'hang', args: null },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );
    await Promise.resolve();

    __testOnly.handleWindowClosedEventForIpcState({
      type: 'window-closed',
      windowId: 'WindowId(99)',
      jsWindowId: null,
    });

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-without-js-close-id',
        raw: { id: 'req-after-null-js-id-close', method: 'sum', args: { ok: true } },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );

    await vi.advanceTimersByTimeAsync(55);
    await staleRequest;

    const responses = native.windowEvalScript.mock.calls.map(([, script]) => parseResponseScript(script));
    expect(responses.find((response) => response.id === 'req-after-null-js-id-close')).toEqual({
      id: 'req-after-null-js-id-close',
      result: { ok: true },
    });
    expect(responses.filter((response) => response.errorCode === 'IPC_IN_FLIGHT_LIMIT')).toHaveLength(0);
  });

  it('reset helper clears stale IPC load and handlers between runtime sessions', async () => {
    vi.useFakeTimers();

    ipcMain.handle('hang', async () => {
      await new Promise(() => {});
      return null;
    });

    const native = createNativeRuntimeMock();
    const staleRequest = __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-reset',
        raw: { id: 'req-old', method: 'hang', args: null },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );
    await Promise.resolve();

    __testOnly.resetIpcRuntimeState();
    ipcMain.handle('sum', (args) => args);

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-reset',
        raw: { id: 'req-new', method: 'sum', args: { restarted: true } },
      },
      { timeoutMs: 50, maxInFlightPerWindow: 1 },
    );

    await vi.advanceTimersByTimeAsync(55);
    await staleRequest;

    const responses = native.windowEvalScript.mock.calls.map(([, script]) => parseResponseScript(script));
    expect(responses.find((response) => response.id === 'req-new')).toEqual({
      id: 'req-new',
      result: { restarted: true },
    });
    expect(ipcMain.hasHandler('hang')).toBe(false);
  });

  it('native window-closed event drives JS BrowserWindow closed timing in dev runtime sync path', () => {
    const win = new BrowserWindow();
    let closed = false;
    win.on('closed', () => {
      closed = true;
    });

    expect(win.isDestroyed()).toBe(false);
    expect(closed).toBe(false);

    __testOnly.syncFrameworkWindowStateFromNativeCloseEvent({
      type: 'window-closed',
      windowId: 'WindowId(77)',
      jsWindowId: win.getId(),
    });

    expect(closed).toBe(true);
    expect(win.isDestroyed()).toBe(true);
  });
});
