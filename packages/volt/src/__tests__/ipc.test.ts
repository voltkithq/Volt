import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { ipcMain, invoke, on, off } from '../ipc.js';

describe('ipcMain', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  beforeEach(() => {
    // Clean up handlers between tests
    // We need to remove all handlers we've added
    // Use getHandler/hasHandler to check, removeHandler to clean
    for (const ch of [
      'test-channel',
      'greet',
      'async-channel',
      'throws',
      'echo',
      'ch1',
      'ch2',
      'timeout-channel',
      'burst',
    ]) {
      if (ipcMain.hasHandler(ch)) {
        ipcMain.removeHandler(ch);
      }
    }
  });

  it('handle registers a handler', () => {
    ipcMain.handle('test-channel', () => 'hello');
    expect(ipcMain.hasHandler('test-channel')).toBe(true);
  });

  it('handle throws on duplicate registration', () => {
    ipcMain.handle('greet', () => 'hi');
    expect(() => ipcMain.handle('greet', () => 'ho')).toThrow(
      'already registered',
    );
  });

  it('handle rejects reserved Volt channels', () => {
    expect(() => ipcMain.handle('volt:native:data.query', () => 'nope')).toThrow(
      'reserved by Volt',
    );
  });

  it('handle rejects reserved internal Volt channels', () => {
    expect(() => ipcMain.handle('__volt_internal:csp-violation', () => 'nope')).toThrow(
      'reserved by Volt',
    );
  });

  it('handle rejects reserved plugin channels', () => {
    expect(() => ipcMain.handle('plugin:acme.search:ping', () => 'nope')).toThrow(
      'reserved by Volt',
    );
  });

  it('removeHandler removes a handler', () => {
    ipcMain.handle('test-channel', () => 'x');
    ipcMain.removeHandler('test-channel');
    expect(ipcMain.hasHandler('test-channel')).toBe(false);
  });

  it('removeHandler is silent for nonexistent channel', () => {
    expect(() => ipcMain.removeHandler('nonexistent')).not.toThrow();
  });

  it('hasHandler returns false for unregistered channels', () => {
    expect(ipcMain.hasHandler('no-such-channel')).toBe(false);
  });

  it('getHandler returns the registered function', () => {
    const fn = () => 42;
    ipcMain.handle('echo', fn);
    expect(ipcMain.getHandler('echo')).toBe(fn);
  });

  it('getHandler returns undefined for unregistered channels', () => {
    expect(ipcMain.getHandler('missing')).toBeUndefined();
  });

  describe('processRequest', () => {
    it('returns result from sync handler', async () => {
      ipcMain.handle('greet', (args) => {
        const a = args as { name: string };
        return `Hello, ${a.name}`;
      });
      const resp = await ipcMain.processRequest('req-1', 'greet', {
        name: 'Alice',
      });
      expect(resp.id).toBe('req-1');
      expect(resp.result).toBe('Hello, Alice');
      expect(resp.error).toBeUndefined();
    });

    it('returns result from async handler', async () => {
      ipcMain.handle('async-channel', async () => {
        return { data: 'async-result' };
      });
      const resp = await ipcMain.processRequest('req-2', 'async-channel', {});
      expect(resp.result).toEqual({ data: 'async-result' });
    });

    it('returns error for unknown channel', async () => {
      const resp = await ipcMain.processRequest('req-3', 'unknown', {});
      expect(resp.error).toContain('Handler not found');
      expect(resp.errorCode).toBe('IPC_HANDLER_NOT_FOUND');
    });

    it('returns error when handler throws', async () => {
      ipcMain.handle('throws', () => {
        throw new Error('Intentional failure');
      });
      const resp = await ipcMain.processRequest('req-4', 'throws', {});
      expect(resp.error).toBe('Intentional failure');
      expect(resp.errorCode).toBe('IPC_HANDLER_ERROR');
      expect(resp.result).toBeUndefined();
    });

    it('returns null for handler that returns undefined', async () => {
      ipcMain.handle('test-channel', () => undefined);
      const resp = await ipcMain.processRequest('req-5', 'test-channel', {});
      expect(resp.result).toBeNull();
    });

    it('returns timeout error code when handler exceeds timeout', async () => {
      vi.useFakeTimers();
      ipcMain.handle('timeout-channel', async () => {
        await new Promise(() => {});
        return 'never';
      });

      const pending = ipcMain.processRequest(
        'req-timeout',
        'timeout-channel',
        {},
        { timeoutMs: 20 },
      );

      await vi.advanceTimersByTimeAsync(25);
      const resp = await pending;
      expect(resp.errorCode).toBe('IPC_HANDLER_TIMEOUT');
      expect(resp.error).toContain('timed out');
      expect(resp.errorDetails).toEqual({ timeoutMs: 20, method: 'timeout-channel' });
    });

    it('handles high request load deterministically', async () => {
      ipcMain.handle('burst', (args) => args);

      const requests = Array.from({ length: 200 }, (_, index) => {
        return ipcMain.processRequest(`req-${index}`, 'burst', { index });
      });

      const responses = await Promise.all(requests);
      expect(responses).toHaveLength(200);
      expect(responses[0].result).toEqual({ index: 0 });
      expect(responses[199].result).toEqual({ index: 199 });
    });
  });
});

describe('renderer-side APIs (invoke, on, off)', () => {
  // In Node.js context without window.__volt__, these should throw
  afterEach(() => {
    const g = globalThis as Record<string, unknown>;
    delete g['window'];
  });

  it('invoke throws in Node.js context', () => {
    expect(() => invoke('test')).toThrow('renderer');
  });

  it('on throws in Node.js context', () => {
    expect(() => on('event', () => {})).toThrow('renderer');
  });

  it('off throws in Node.js context', () => {
    expect(() => off('event', () => {})).toThrow('renderer');
  });

  it('invoke forwards null payload for zero args', async () => {
    const bridge = {
      invoke: vi.fn(async () => 'ok'),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    await invoke('ping');

    expect(bridge.invoke).toHaveBeenCalledWith('ping', null);
  });

  it('invoke forwards direct payload for a single argument', async () => {
    const bridge = {
      invoke: vi.fn(async () => 'ok'),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    await invoke('single', { foo: true });

    expect(bridge.invoke).toHaveBeenCalledWith('single', { foo: true });
  });

  it('invoke forwards full args array for variadic arguments', async () => {
    const bridge = {
      invoke: vi.fn(async () => 'ok'),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    await invoke('sum', 1, 2, 3);

    expect(bridge.invoke).toHaveBeenCalledWith('sum', [1, 2, 3]);
  });

  it('on delegates to renderer bridge when available', () => {
    const handler = vi.fn();
    const bridge = {
      invoke: vi.fn(async () => 'ok'),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    on('demo:event', handler);

    expect(bridge.on).toHaveBeenCalledWith('demo:event', handler);
  });

  it('off delegates to renderer bridge when available', () => {
    const handler = vi.fn();
    const bridge = {
      invoke: vi.fn(async () => 'ok'),
      on: vi.fn(),
      off: vi.fn(),
    };
    (globalThis as Record<string, unknown>)['window'] = { __volt__: bridge };

    off('demo:event', handler);

    expect(bridge.off).toHaveBeenCalledWith('demo:event', handler);
  });
});
