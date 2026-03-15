import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { pluginInitCommand } from '../../commands/plugin/init.js';
import { validatePluginManifest } from '../../utils/plugin-manifest.js';
import { createTempWorkspace, readJson } from './fixtures.js';

describe('plugin init', () => {
  it('creates the expected scaffold with a valid manifest', async () => {
    const cwd = createTempWorkspace();
    await pluginInitCommand(
      'new-plugin',
      { cwd },
      async () => ({
        pluginId: 'com.example.newplugin',
        name: 'New Plugin',
        description: 'Test plugin',
        capabilities: ['fs', 'http'],
      }),
    );

    const projectDir = resolve(cwd, 'new-plugin');
    expect(existsSync(resolve(projectDir, 'volt-plugin.json'))).toBe(true);
    expect(existsSync(resolve(projectDir, 'src', 'plugin.ts'))).toBe(true);
    expect(existsSync(resolve(projectDir, 'package.json'))).toBe(true);
    expect(existsSync(resolve(projectDir, 'tsconfig.json'))).toBe(true);
    expect(
      validatePluginManifest(readJson(resolve(projectDir, 'volt-plugin.json'))).valid,
    ).toBe(true);
    expect(readFileSync(resolve(projectDir, 'src', 'plugin.ts'), 'utf8')).toContain(
      "definePlugin({",
    );
  });
});
