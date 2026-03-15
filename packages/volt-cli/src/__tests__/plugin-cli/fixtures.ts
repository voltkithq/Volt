import { cpSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import type { Permission } from 'voltkit';
import { createPluginScaffold } from '../../commands/plugin/scaffold.js';

export function createTempWorkspace(prefix = 'volt-plugin-cli-'): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

export function createTempPluginProject(
  name = 'sample-plugin',
  overrides: Partial<{ pluginId: string; capabilities: Permission[]; description: string }> = {},
): string {
  const root = createTempWorkspace();
  const projectDir = resolve(root, name);
  createPluginScaffold({
    targetDir: projectDir,
    pluginId: overrides.pluginId ?? 'com.example.sample',
    name: 'Sample Plugin',
    description: overrides.description ?? 'Sample plugin',
    capabilities: overrides.capabilities ?? ['fs'],
  });
  return projectDir;
}

export function copyPluginProject(source: string, target: string): void {
  cpSync(source, target, { recursive: true });
}

export function readJson(path: string): unknown {
  return JSON.parse(readFileSync(path, 'utf8')) as unknown;
}

export function writePluginManifest(projectDir: string, update: (value: Record<string, unknown>) => Record<string, unknown>): void {
  const manifestPath = resolve(projectDir, 'volt-plugin.json');
  const manifest = readJson(manifestPath) as Record<string, unknown>;
  writeFileSync(manifestPath, `${JSON.stringify(update(manifest), null, 2)}\n`, 'utf8');
}

export function writeVoltAppConfig(appRoot: string, contents: string): void {
  writeFileSync(resolve(appRoot, 'volt.config.mjs'), contents, 'utf8');
}
