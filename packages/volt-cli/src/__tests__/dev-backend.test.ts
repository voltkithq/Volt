import { existsSync, mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { ipcMain } from 'voltkit';
import { __testOnly, loadBackendEntrypointForDev } from '../commands/dev/backend.js';
import { resetIpcRuntimeState } from '../commands/dev/ipc.js';
import {
  configureRuntimeModuleState,
  resetRuntimeModuleState,
} from '../commands/dev/runtime-modules/shared.js';

interface TempProject {
  rootDir: string;
  cleanup: () => void;
}

function createTempProject(files: Record<string, string>): TempProject {
  const rootDir = mkdtempSync(join(process.cwd(), '.tmp-dev-backend-test-'));
  for (const [relativePath, contents] of Object.entries(files)) {
    const absolutePath = join(rootDir, relativePath);
    mkdirSync(dirname(absolutePath), { recursive: true });
    writeFileSync(absolutePath, contents, 'utf8');
  }
  return {
    rootDir,
    cleanup: () => {
      rmSync(rootDir, { recursive: true, force: true });
    },
  };
}

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

  it('loads the volt:bench dev shim for backend handlers', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        import * as bench from 'volt:bench';

        ipcMain.handle('dev-backend:bench', async () => {
          const profile = await bench.analyticsProfile({ datasetSize: 1_200 });
          const workflow = await bench.runWorkflowBenchmark({ batchSize: 800, passes: 2 });
          return {
            datasetSize: profile.datasetSize,
            batchSize: workflow.batchSize,
            pipelineLength: workflow.pipeline.length,
          };
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

    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: vi.fn() },
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    const response = await ipcMain.processRequest(
      'req-bench',
      'dev-backend:bench',
      null,
      { timeoutMs: 200 },
    );
    expect(response).toEqual({
      id: 'req-bench',
      result: {
        datasetSize: 1_200,
        batchSize: 800,
        pipelineLength: 5,
      },
    });
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
