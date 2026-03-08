import { describe, it, expect } from 'vitest';
import { defineConfig } from '../types.js';

describe('defineConfig', () => {
  it('returns the config object as-is (passthrough)', () => {
    const config = { name: 'My App', version: '1.0.0' };
    const result = defineConfig(config);
    expect(result).toBe(config);
    expect(result.name).toBe('My App');
    expect(result.version).toBe('1.0.0');
  });

  it('preserves all config fields', () => {
    const config = defineConfig({
      name: 'Full App',
      version: '2.0.0',
      permissions: ['clipboard', 'fs'],
      window: { width: 1024, height: 768 },
      build: { outDir: 'build' },
      package: { identifier: 'com.test.app' },
      updater: {
        endpoint: 'https://updates.example.com',
        publicKey: 'abc123',
      },
      runtime: { poolSize: 3 },
      devtools: false,
    });
    expect(config.name).toBe('Full App');
    expect(config.permissions).toEqual(['clipboard', 'fs']);
    expect(config.window?.width).toBe(1024);
    expect(config.build?.outDir).toBe('build');
    expect(config.package?.identifier).toBe('com.test.app');
    expect(config.updater?.endpoint).toBe('https://updates.example.com');
    expect(config.runtime?.poolSize).toBe(3);
    expect(config.devtools).toBe(false);
  });

  it('exposes ambient volt:* module declarations for backend code', () => {
    const acceptTypes = (
      _ipc: import('volt:ipc').IpcMain,
      _eventsEmit: typeof import('volt:events').emit,
      _windowQuit: typeof import('volt:window').quit,
      _menuSetAppMenu: typeof import('volt:menu').setAppMenu,
      _globalShortcutRegister: typeof import('volt:globalShortcut').register,
      _trayCreate: typeof import('volt:tray').create,
      _dbOpen: typeof import('volt:db').open,
      _secureStorageSet: typeof import('volt:secureStorage').set,
      _clipboardReadText: typeof import('volt:clipboard').readText,
      _cryptoSha256: typeof import('volt:crypto').sha256,
      _osPlatform: typeof import('volt:os').platform,
      _shellOpenExternal: typeof import('volt:shell').openExternal,
      _notificationShow: typeof import('volt:notification').show,
      _dialogShowMessage: typeof import('volt:dialog').showMessage,
      _fsReadFile: typeof import('volt:fs').readFile,
      _httpFetch: typeof import('volt:http').fetch,
      _updaterCheckForUpdate: typeof import('volt:updater').checkForUpdate,
    ): void => {
      void _ipc;
      void _eventsEmit;
      void _windowQuit;
      void _menuSetAppMenu;
      void _globalShortcutRegister;
      void _trayCreate;
      void _dbOpen;
      void _secureStorageSet;
      void _clipboardReadText;
      void _cryptoSha256;
      void _osPlatform;
      void _shellOpenExternal;
      void _notificationShow;
      void _dialogShowMessage;
      void _fsReadFile;
      void _httpFetch;
      void _updaterCheckForUpdate;
    };

    const assertHttpResponseShape = async (
      fetchFn: typeof import('volt:http').fetch,
    ): Promise<void> => {
      const response = await fetchFn({ url: 'https://example.com' });
      const _headers: Record<string, string[]> = response.headers;
      const _text: string = await response.text();
      const _json: unknown = await response.json();
      void _headers;
      void _text;
      void _json;
    };

    expect(typeof acceptTypes).toBe('function');
    expect(typeof assertHttpResponseShape).toBe('function');
  });
});
