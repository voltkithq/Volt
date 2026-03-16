import { existsSync, utimesSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { ipcMain } from 'voltkit';
import { __testOnly, loadBackendEntrypointForDev } from '../commands/dev/backend.js';
import { resetIpcRuntimeState } from '../commands/dev/ipc.js';
import {
  configureRuntimeModuleState,
  resetRuntimeModuleState,
} from '../commands/dev/runtime-modules/shared.js';
import { createTempProject } from './dev-backend-test-utils.js';

describe('dev backend bootstrap', () => {
  const cleanups: Array<() => void> = [];

  beforeEach(() => {
    resetIpcRuntimeState();
    resetRuntimeModuleState();
  });

  afterEach(() => {
    for (const cleanup of cleanups.splice(0)) {
      cleanup();
    }
    resetIpcRuntimeState();
    resetRuntimeModuleState();
  });

  it('loads backend entry and registers IPC handlers in dev mode', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        ipcMain.handle('dev-backend:ping', () => ({ ok: true }));
      `,
      'tsconfig.json': JSON.stringify({
        compilerOptions: {
          target: 'ES2022',
          module: 'ESNext',
          moduleResolution: 'bundler',
          strict: true,
        },
      }),
    });
    cleanups.push(project.cleanup);

    const evalScript = vi.fn();
    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: evalScript },
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    expect(backendLoadState.loaded).toBe(true);

    const response = await ipcMain.processRequest(
      'req-backend',
      'dev-backend:ping',
      null,
      { timeoutMs: 200 },
    );
    expect(response).toEqual({
      id: 'req-backend',
      result: { ok: true },
    });
  });

  it('supports ipcMain.emit through the dev runtime shim', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        ipcMain.handle('dev-backend:emit', () => {
          ipcMain.emit('demo:event', { ok: true });
          return { emitted: true };
        });
      `,
      'tsconfig.json': JSON.stringify({
        compilerOptions: {
          target: 'ES2022',
          module: 'ESNext',
          moduleResolution: 'bundler',
          strict: true,
        },
      }),
    });
    cleanups.push(project.cleanup);

    const evalScript = vi.fn();
    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: evalScript },
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    const response = await ipcMain.processRequest(
      'req-emit',
      'dev-backend:emit',
      null,
      { timeoutMs: 200 },
    );
    expect(response).toEqual({
      id: 'req-emit',
      result: { emitted: true },
    });

    expect(evalScript).toHaveBeenCalledTimes(1);
    const [windowId, script] = evalScript.mock.calls[0];
    expect(windowId).toBe('window-dev');
    expect(script).toContain('window.__volt_event__');
    expect(script).toContain('demo:event');
  });

  it('fails fast when backend imports unsupported volt:* modules', async () => {
    const project = createTempProject({
      'backend.ts': `
        import 'volt:unknown-module';
      `,
      'tsconfig.json': JSON.stringify({
        compilerOptions: {
          target: 'ES2022',
          module: 'ESNext',
          moduleResolution: 'bundler',
          strict: true,
        },
      }),
    });
    cleanups.push(project.cleanup);

    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: vi.fn() },
    });

    await expect(
      loadBackendEntrypointForDev(project.rootDir, './backend.ts'),
    ).rejects.toThrow('Unsupported backend module in dev mode: volt:unknown-module');
  });

  it('clears existing IPC handlers before backend reload', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        ipcMain.handle('dev-backend:reloadable', () => ({ version: 1 }));
      `,
      'tsconfig.json': JSON.stringify({
        compilerOptions: {
          target: 'ES2022',
          module: 'ESNext',
          moduleResolution: 'bundler',
          strict: true,
        },
      }),
    });
    cleanups.push(project.cleanup);

    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: vi.fn() },
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    const initial = await ipcMain.processRequest(
      'req-reload-1',
      'dev-backend:reloadable',
      null,
      { timeoutMs: 200 },
    );
    expect(initial).toEqual({
      id: 'req-reload-1',
      result: { version: 1 },
    });

    writeFileSync(join(project.rootDir, 'backend.ts'), `
      import { ipcMain } from 'volt:ipc';
      ipcMain.handle('dev-backend:reloadable', () => ({ version: 2 }));
    `, 'utf8');

    const reloadedState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(reloadedState.dispose);

    const reloaded = await ipcMain.processRequest(
      'req-reload-2',
      'dev-backend:reloadable',
      null,
      { timeoutMs: 200 },
    );
    expect(reloaded).toEqual({
      id: 'req-reload-2',
      result: { version: 2 },
    });
  });

  it('prunes stale dev backend bundles while preserving fresh ones', () => {
    const project = createTempProject({});
    cleanups.push(project.cleanup);

    const backendTempRoot = join(project.rootDir, '.volt-dev', 'dev-backend');
    const staleBundleDir = __testOnly.createScopedTempDirectory(
      backendTempRoot,
      __testOnly.DEV_BACKEND_BUNDLE_PREFIX,
    );
    const freshBundleDir = __testOnly.createScopedTempDirectory(
      backendTempRoot,
      __testOnly.DEV_BACKEND_BUNDLE_PREFIX,
    );

    const nowMs = Date.now();
    const staleTimestamp = new Date(nowMs - (__testOnly.DEV_BACKEND_STALE_BUNDLE_MAX_AGE_MS + 60_000));
    utimesSync(staleBundleDir, staleTimestamp, staleTimestamp);

    const recovery = __testOnly.recoverStaleScopedDirectories(backendTempRoot, {
      prefix: __testOnly.DEV_BACKEND_BUNDLE_PREFIX,
      staleAfterMs: __testOnly.DEV_BACKEND_STALE_BUNDLE_MAX_AGE_MS,
      nowMs,
    });

    expect(recovery.removed).toBe(1);
    expect(recovery.failures).toBe(0);
    expect(existsSync(staleBundleDir)).toBe(false);
    expect(existsSync(freshBundleDir)).toBe(true);
  });
});
