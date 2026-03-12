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
      _benchAnalyticsProfile: typeof import('volt:bench').analyticsProfile,
      _benchRunAnalyticsBenchmark: typeof import('volt:bench').runAnalyticsBenchmark,
      _benchRunWorkflowBenchmark: typeof import('volt:bench').runWorkflowBenchmark,
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
      void _benchAnalyticsProfile;
      void _benchRunAnalyticsBenchmark;
      void _benchRunWorkflowBenchmark;
      void _updaterCheckForUpdate;
    };

    const assertBenchResponseShape = async (
      profileFn: typeof import('volt:bench').analyticsProfile,
      analyticsFn: typeof import('volt:bench').runAnalyticsBenchmark,
      workflowFn: typeof import('volt:bench').runWorkflowBenchmark,
    ): Promise<void> => {
      const profile = await profileFn({ datasetSize: 1_200 });
      const analytics = await analyticsFn({ datasetSize: 2_400, iterations: 2, searchTerm: 'risk' });
      const workflow = await workflowFn({ batchSize: 750, passes: 2, pipeline: ['normalizeText', 'buildDigests'] });

      const _cachedSizes: number[] = profile.cachedSizes;
      const _profileSpread: Record<string, number> = profile.categorySpread;
      const _analyticsWinner: string = analytics.categoryWinners[0]?.category ?? '';
      const _analyticsSampleTitle: string = analytics.sample[0]?.title ?? '';
      const _workflowStepDuration: number = workflow.stepTimings[0]?.durationMs ?? 0;
      const _workflowDigest: string = workflow.digestSample[0] ?? '';

      void _cachedSizes;
      void _profileSpread;
      void _analyticsWinner;
      void _analyticsSampleTitle;
      void _workflowStepDuration;
      void _workflowDigest;
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
    expect(typeof assertBenchResponseShape).toBe('function');
    expect(typeof assertHttpResponseShape).toBe('function');
  });
});
