import { mkdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { startPluginDevelopment } from '../../commands/dev/plugin-development.js';
import { copyPluginProject, createTempPluginProject, createTempWorkspace, writeVoltAppConfig } from './fixtures.js';

describe('plugin dev watch', () => {
  it('resolves configured plugin projects and restarts on rebuild', async () => {
    const appRoot = createTempWorkspace();
    const pluginDir = resolve(appRoot, 'plugins', 'watched-plugin');
    mkdirSync(resolve(appRoot, 'plugins'), { recursive: true });
    const sourceProject = createTempPluginProject('watched-plugin');
    copyPluginProject(sourceProject, pluginDir);
    writeVoltAppConfig(
      appRoot,
      `export default { name: 'App', plugins: { pluginDirs: ['./plugins'], enabled: ['com.example.sample'] } };`,
    );

    const shutdown = vi.fn(async () => {});
    let rebuildHandler: ((ok: boolean) => Promise<void> | void) | null = null;

    const dispose = await startPluginDevelopment(
      appRoot,
      { pluginDirs: ['./plugins'], enabled: ['com.example.sample'] },
      {
        buildPlugin: vi.fn(async () => {}),
        loadProject: (cwd) => ({
          projectRoot: cwd,
          manifestPath: resolve(cwd, 'volt-plugin.json'),
          manifestResult: { valid: true, errors: [], manifest: JSON.parse(readFileSync(resolve(cwd, 'volt-plugin.json'), 'utf8')) },
          manifest: JSON.parse(readFileSync(resolve(cwd, 'volt-plugin.json'), 'utf8')),
        }),
        startHarness: vi.fn(async () => ({
          process: {} as never,
          state: { commands: new Set(), eventSubscriptions: new Set(), ipcHandlers: new Set(), emittedEvents: [] },
          activate: vi.fn(async () => {}),
          invokeCommand: vi.fn(async () => null),
          shutdown,
          kill: vi.fn(),
        })),
        createDataRoot: () => '/tmp/plugin-data',
        watchPlugin: vi.fn(async (_cwd, onRebuild) => {
          rebuildHandler = onRebuild;
          return async () => {};
        }),
        logger: { log: vi.fn(), warn: vi.fn(), error: vi.fn() },
      },
    );

    await rebuildHandler?.(true);
    await dispose();

    expect(shutdown).toHaveBeenCalledTimes(2);
  });
});
