import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { type Permission } from 'voltkit';
import { resolveVoltCliVersion, toSafePackageName } from './project.js';

export interface PluginScaffoldOptions {
  targetDir: string;
  pluginId: string;
  name: string;
  description: string;
  capabilities: Permission[];
}

export function createPluginScaffold(options: PluginScaffoldOptions): void {
  const targetDir = resolve(options.targetDir);
  const srcDir = join(targetDir, 'src');
  mkdirSync(srcDir, { recursive: true });

  writeFileSync(join(targetDir, 'volt-plugin.json'), `${JSON.stringify(createManifest(options), null, 2)}\n`, 'utf8');
  writeFileSync(join(srcDir, 'plugin.ts'), createPluginSource(options), 'utf8');
  writeFileSync(join(targetDir, 'package.json'), `${JSON.stringify(createPackageJson(options), null, 2)}\n`, 'utf8');
  writeFileSync(join(targetDir, 'tsconfig.json'), `${JSON.stringify(createTsconfig(), null, 2)}\n`, 'utf8');
}

function createManifest(options: PluginScaffoldOptions) {
  return {
    id: options.pluginId,
    name: options.name,
    description: options.description,
    version: '0.1.0',
    apiVersion: 1,
    engine: {
      volt: `>=${resolveVoltCliVersion()}`,
    },
    backend: './dist/plugin.js',
    capabilities: options.capabilities,
    contributes: {
      commands: [],
    },
  };
}

function createPluginSource(options: PluginScaffoldOptions): string {
  const logLine =
    options.description.trim().length > 0
      ? `  context.log.info(${JSON.stringify(`${options.name} activated`)})`
      : `  context.log.info(${JSON.stringify(`${options.pluginId} activated`)})`;
  return [
    "import { definePlugin } from 'volt:plugin';",
    '',
    'definePlugin({',
    '  async activate(context) {',
    `    ${logLine};`,
    '  },',
    '  async deactivate(context) {',
    "    context.log.info('plugin deactivated');",
    '  },',
    '});',
    '',
  ].join('\n');
}

function createPackageJson(options: PluginScaffoldOptions) {
  return {
    name: toSafePackageName(options.name || options.pluginId),
    version: '0.1.0',
    private: true,
    type: 'module',
    scripts: {
      build: 'volt plugin build',
      test: 'volt plugin test',
      doctor: 'volt plugin doctor',
    },
    dependencies: {
      voltkit: `^${resolveVoltCliVersion()}`,
    },
    devDependencies: {
      typescript: '^5.9.3',
    },
  };
}

function createTsconfig() {
  return {
    compilerOptions: {
      target: 'ES2022',
      module: 'ES2022',
      moduleResolution: 'bundler',
      strict: true,
      isolatedModules: true,
      skipLibCheck: true,
      noEmit: true,
      types: [],
    },
    include: ['src/**/*.ts'],
  };
}
