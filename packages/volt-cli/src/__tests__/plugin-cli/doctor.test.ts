import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { pluginBuildCommand } from '../../commands/plugin/build.js';
import { collectPluginDoctorChecks } from '../../commands/plugin/doctor.js';
import { copyPluginProject, createTempPluginProject, createTempWorkspace, writePluginManifest, writeVoltAppConfig } from './fixtures.js';

describe('plugin doctor', () => {
  it('reports passing checks for a valid built plugin inside a compatible app', async () => {
    const appRoot = createTempWorkspace();
    const pluginDir = resolve(appRoot, 'plugins', 'sample-plugin');
    mkdirSync(resolve(appRoot, 'plugins'), { recursive: true });
    mkdirSync(pluginDir, { recursive: true });
    const scaffoldRoot = createTempPluginProject('sample-plugin');
    copyPluginProject(scaffoldRoot, pluginDir);
    writeVoltAppConfig(
      appRoot,
      `export default { name: 'App', permissions: ['fs'], plugins: { grants: { 'com.example.sample': ['fs'] } } };`,
    );
    await pluginBuildCommand({ cwd: pluginDir });

    const checks = await collectPluginDoctorChecks(pluginDir);

    expect(checks.every((check) => check.status !== 'fail')).toBe(true);
    expect(checks.find((check) => check.id === 'host.permissions')?.status).toBe('pass');
  });

  it('fails when engine range or apiVersion is incompatible', async () => {
    const projectDir = createTempPluginProject();
    writePluginManifest(projectDir, (manifest) => ({
      ...manifest,
      apiVersion: 99,
      engine: { volt: '>=9.0.0' },
    }));

    const checks = await collectPluginDoctorChecks(projectDir);

    expect(checks.find((check) => check.id === 'api.version')?.status).toBe('fail');
    expect(checks.find((check) => check.id === 'engine.volt')?.status).toBe('fail');
  });

  it('reports invalid manifest schema as a failed doctor check', async () => {
    const projectDir = createTempPluginProject();
    writePluginManifest(projectDir, (manifest) => ({
      ...manifest,
      id: 'not-a-reverse-domain',
    }));

    const checks = await collectPluginDoctorChecks(projectDir);

    expect(checks).toHaveLength(1);
    expect(checks[0]).toMatchObject({
      id: 'manifest.schema',
      status: 'fail',
    });
  });
});
