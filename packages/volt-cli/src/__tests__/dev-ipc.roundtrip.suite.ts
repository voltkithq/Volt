import { describe, expect, it, vi } from 'vitest';
import { ipcMain } from 'voltkit';
import { __testOnly } from '../commands/dev.js';
import {
  createNativeRuntimeMock,
  parseResponseScript,
  setupDevIpcTestLifecycle,
} from './dev-ipc.shared.js';

describe('dev IPC round-trip: routing and errors', () => {
  setupDevIpcTestLifecycle();

  it('handles a single IPC request and sends response script to the correct window', async () => {
    ipcMain.handle('sum', (args) => {
      const input = args as { a: number; b: number };
      return { total: input.a + input.b };
    });

    const native = createNativeRuntimeMock();

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-1',
        raw: { id: 'req-1', method: 'sum', args: { a: 2, b: 3 } },
      },
      { timeoutMs: 200 },
    );

    expect(native.windowEvalScript).toHaveBeenCalledTimes(1);
    const [windowId, script] = native.windowEvalScript.mock.calls[0];
    expect(windowId).toBe('window-1');
    const response = parseResponseScript(script);
    expect(response).toEqual({ id: 'req-1', result: { total: 5 } });
  });

  it('handles string-encoded native IPC payloads', async () => {
    ipcMain.handle('sum', (args) => {
      const input = args as { a: number; b: number };
      return { total: input.a + input.b };
    });

    const native = createNativeRuntimeMock();
    const raw = JSON.stringify({ id: 'req-string', method: 'sum', args: { a: 4, b: 6 } });

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-encoded',
        raw,
      },
      { timeoutMs: 200 },
    );

    expect(native.windowEvalScript).toHaveBeenCalledTimes(1);
    const [windowId, script] = native.windowEvalScript.mock.calls[0];
    expect(windowId).toBe('window-encoded');
    expect(parseResponseScript(script)).toEqual({
      id: 'req-string',
      result: { total: 10 },
    });
  });

  it('escapes </script> sequences in IPC response scripts', () => {
    const script = __testOnly.createIpcResponseScript({
      id: 'req-script',
      result: '</script><script>alert(1)</script>',
    });
    expect(script).not.toContain('</script>');
    expect(parseResponseScript(script).result).toBe('</script><script>alert(1)</script>');
  });

  it('handles high-load burst deterministically', async () => {
    ipcMain.handle('echo-load', (args) => args);

    const native = createNativeRuntimeMock();
    const requests = Array.from({ length: 100 }, (_, index) => {
      const id = `load-${index}`;
      return __testOnly.handleIpcMessageEvent(
        native,
        {
          type: 'ipc-message',
          windowId: 'window-load',
          raw: { id, method: 'echo-load', args: { index } },
        },
        { timeoutMs: 200, maxInFlightPerWindow: 1000 },
      );
    });

    await Promise.all(requests);
    expect(native.windowEvalScript).toHaveBeenCalledTimes(100);

    const ids = native.windowEvalScript.mock.calls.map(([, script]) => parseResponseScript(script).id);
    expect(ids).toContain('load-0');
    expect(ids).toContain('load-99');
    expect(new Set(ids).size).toBe(100);
  });

  it('returns timeout error response deterministically', async () => {
    vi.useFakeTimers();

    ipcMain.handle('hang', async () => {
      await new Promise(() => {});
      return { unreachable: true };
    });

    const native = createNativeRuntimeMock();
    const pending = __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-timeout',
        raw: { id: 'req-timeout', method: 'hang', args: null },
      },
      { timeoutMs: 50 },
    );

    await vi.advanceTimersByTimeAsync(55);
    await pending;

    expect(native.windowEvalScript).toHaveBeenCalledTimes(1);
    const response = parseResponseScript(native.windowEvalScript.mock.calls[0][1]);
    expect(response.id).toBe('req-timeout');
    expect(response.errorCode).toBe('IPC_HANDLER_TIMEOUT');
    expect(response.error).toContain('timed out');
  });

  it('rejects oversized IPC payloads with a stable error code', async () => {
    ipcMain.handle('sum', (args) => args);
    const native = createNativeRuntimeMock();

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-size',
        raw: {
          id: 'req-oversized',
          method: 'sum',
          args: { data: 'x'.repeat(2048) },
        },
      },
      { maxPayloadBytes: 256 },
    );

    expect(native.windowEvalScript).toHaveBeenCalledTimes(1);
    const response = parseResponseScript(native.windowEvalScript.mock.calls[0][1]);
    expect(response.id).toBe('req-oversized');
    expect(response.errorCode).toBe('IPC_PAYLOAD_TOO_LARGE');
    expect(response.error).toContain('payload too large');
  });

  it('returns an explicit error for malformed IPC payloads instead of hanging', async () => {
    const native = createNativeRuntimeMock();

    await __testOnly.handleIpcMessageEvent(
      native,
      {
        type: 'ipc-message',
        windowId: 'window-malformed',
        raw: { method: 'sum', args: { a: 1, b: 2 } },
      },
      { timeoutMs: 200 },
    );

    expect(native.windowEvalScript).toHaveBeenCalledTimes(1);
    const response = parseResponseScript(native.windowEvalScript.mock.calls[0][1]);
    expect(response.id).toBe('unknown');
    expect(response.errorCode).toBe('IPC_HANDLER_ERROR');
    expect(response.error).toContain('Invalid IPC request payload');
  });

  it('parses legacy ipc-message events that use jsWindowId/payload fields', () => {
    const parsed = __testOnly.parseNativeEvent({
      type: 'ipc-message',
      jsWindowId: 'window-legacy',
      payload: { id: 'req-legacy', method: 'ping', args: null },
    });

    expect(parsed).toEqual({
      type: 'ipc-message',
      windowId: 'window-legacy',
      raw: { id: 'req-legacy', method: 'ping', args: null },
    });
  });

  it('parses snake_case native event payloads from older runtimes', () => {
    const parsed = __testOnly.parseNativeEvent({
      type: 'ipc_message',
      window_id: 'window-legacy-2',
      raw: { id: 'req-legacy-2', method: 'ping', args: null },
    });

    expect(parsed).toEqual({
      type: 'ipc-message',
      windowId: 'window-legacy-2',
      raw: { id: 'req-legacy-2', method: 'ping', args: null },
    });
  });
});
