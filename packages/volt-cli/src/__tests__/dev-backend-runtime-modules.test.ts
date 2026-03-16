import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ipcMain } from 'voltkit';
import { loadBackendEntrypointForDev } from '../commands/dev/backend.js';
import { resetIpcRuntimeState } from '../commands/dev/ipc.js';
import {
  configureRuntimeModuleState,
  resetRuntimeModuleState,
} from '../commands/dev/runtime-modules/shared.js';
import { createTempProject } from './dev-backend-test-utils.js';

describe('dev backend runtime modules', () => {
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

  it('loads the volt:plugins dev shim for backend handlers', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        import * as plugins from 'volt:plugins';

        ipcMain.handle('dev-backend:plugins', async () => {
          const states = await plugins.getStates();
          const issues = await plugins.getDiscoveryIssues();
          return {
            stateCount: states.length,
            issueCount: issues.length,
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
      'req-plugins',
      'dev-backend:plugins',
      null,
      { timeoutMs: 200 },
    );
    expect(response).toEqual({
      id: 'req-plugins',
      result: {
        stateCount: 0,
        issueCount: 0,
      },
    });
  });

  it('enforces permissions in clipboard, shell, and secureStorage dev shims', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        import * as clipboard from 'volt:clipboard';
        import * as shell from 'volt:shell';
        import * as secureStorage from 'volt:secureStorage';

        ipcMain.handle('dev-backend:permissions', async () => {
          const failures: string[] = [];

          try {
            clipboard.readText();
          } catch (error) {
            failures.push(String(error));
          }

          try {
            shell.showItemInFolder('C:/tmp/example.txt');
          } catch (error) {
            failures.push(String(error));
          }

          try {
            await secureStorage.has('token');
          } catch (error) {
            failures.push(String(error));
          }

          return failures;
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
      permissions: [],
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    const response = await ipcMain.processRequest(
      'req-permissions',
      'dev-backend:permissions',
      null,
      { timeoutMs: 200 },
    );
    expect(response).toEqual({
      id: 'req-permissions',
      result: [
        "Error: [volt:clipboard] Permission denied: clipboard.readText() requires 'clipboard' in volt.config.ts permissions.",
        "Error: [volt:shell] Permission denied: shell.showItemInFolder() requires 'shell' in volt.config.ts permissions.",
        "Error: [volt:secureStorage] Permission denied: secureStorage.has() requires 'secureStorage' in volt.config.ts permissions.",
      ],
    });
  });

  it('enforces permissions and SSRF protections in the dev http shim', async () => {
    const project = createTempProject({
      'backend.ts': `
        import { ipcMain } from 'volt:ipc';
        import { fetch } from 'volt:http';

        ipcMain.handle('dev-backend:http-guard', async () => {
          const denied = await fetch({ url: 'https://example.com' })
            .then(() => 'unexpected')
            .catch((error) => String(error));

          return { denied };
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
      permissions: [],
    });

    const backendLoadState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(backendLoadState.dispose);

    const denied = await ipcMain.processRequest(
      'req-http-denied',
      'dev-backend:http-guard',
      null,
      { timeoutMs: 200 },
    );
    expect(denied).toEqual({
      id: 'req-http-denied',
      result: {
        denied: "Error: [volt:http] Permission denied: http.fetch() requires 'http' in volt.config.ts permissions.",
      },
    });

    writeFileSync(
      join(project.rootDir, 'backend.ts'),
      `
      import { ipcMain } from 'volt:ipc';
      import { fetch } from 'volt:http';

      ipcMain.handle('dev-backend:http-guard', async () => {
        const blocked = await fetch({ url: 'http://127.0.0.1:8080' })
          .then(() => 'unexpected')
          .catch((error) => String(error));

        return { blocked };
      });
    `,
      'utf8',
    );

    configureRuntimeModuleState({
      projectRoot: project.rootDir,
      defaultWindowId: 'window-dev',
      nativeRuntime: { windowEvalScript: vi.fn() },
      permissions: ['http'],
    });
    resetIpcRuntimeState();
    const reloadedState = await loadBackendEntrypointForDev(project.rootDir, './backend.ts');
    cleanups.push(reloadedState.dispose);

    const blocked = await ipcMain.processRequest(
      'req-http-blocked',
      'dev-backend:http-guard',
      null,
      { timeoutMs: 200 },
    );
    expect(blocked).toEqual({
      id: 'req-http-blocked',
      result: {
        blocked: "Error: [volt:http] HTTP request blocked in dev mode: host '127.0.0.1' is not allowed.",
      },
    });
  });
});
