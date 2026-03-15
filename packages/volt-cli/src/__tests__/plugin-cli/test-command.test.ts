import { describe, expect, it, vi } from 'vitest';
import { pluginTestCommand } from '../../commands/plugin/test.js';
import { createTempPluginProject } from './fixtures.js';

describe('plugin test command', () => {
  it('builds, activates, and invokes contributed commands', async () => {
    const projectDir = createTempPluginProject();
    const buildPlugin = vi.fn(async () => {});
    const activate = vi.fn(async () => {});
    const invokeCommand = vi.fn(async () => ({ ok: true }));
    const shutdown = vi.fn(async () => {});

    await pluginTestCommand(
      {
        buildPlugin,
        createDataRoot: () => '/tmp/plugin-data',
        startHarness: vi.fn(async () => ({
          process: {} as never,
          state: { commands: new Set(), eventSubscriptions: new Set(), ipcHandlers: new Set(), emittedEvents: [] },
          activate,
          invokeCommand,
          shutdown,
          kill: vi.fn(),
        })),
      },
      { cwd: projectDir },
    );

    expect(buildPlugin).toHaveBeenCalledWith(projectDir);
    expect(activate).toHaveBeenCalled();
    expect(shutdown).toHaveBeenCalled();
  });
});
