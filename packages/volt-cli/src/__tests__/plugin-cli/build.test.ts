import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { pluginBuildCommand } from '../../commands/plugin/build.js';
import { createTempPluginProject } from './fixtures.js';

describe('plugin build', () => {
  it('bundles the plugin source to dist/plugin.js', async () => {
    const projectDir = createTempPluginProject();

    await pluginBuildCommand({ cwd: projectDir });

    const bundlePath = resolve(projectDir, 'dist', 'plugin.js');
    expect(existsSync(bundlePath)).toBe(true);
    expect(readFileSync(bundlePath, 'utf8')).toContain('definePlugin');
  });
});
