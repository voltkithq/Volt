import { chmodSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { packageCommand } from '../commands/package.js';
import { writeRuntimeArtifactManifest } from '../utils/runtime-artifact.js';

const tempDirs: string[] = [];

function createTempProjectDir(): string {
  const dir = mkdtempSync(join(tmpdir(), 'volt-package-linux-ci-'));
  tempDirs.push(dir);
  writeFileSync(
    join(dir, 'volt.config.mjs'),
    `export default {
  name: 'My App',
  version: '0.1.0',
  window: { width: 800, height: 600, title: 'My App' }
};
`,
    'utf8',
  );
  return dir;
}

function createFakeLinuxTool(binDir: string, name: string, scriptBody: string): void {
  const toolPath = join(binDir, name);
  const script = `#!/usr/bin/env node\n${scriptBody}\n`;
  writeFileSync(toolPath, script, 'utf8');
  chmodSync(toolPath, 0o755);
}

function installFakeLinuxPackagingTools(projectDir: string): string {
  const binDir = join(projectDir, '.fake-bin');
  mkdirSync(binDir, { recursive: true });
  createFakeLinuxTool(
    binDir,
    'appimagetool',
    `import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
const output = process.argv[3];
if (!output) {
  process.exit(2);
}
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, '');
`,
  );
  createFakeLinuxTool(
    binDir,
    'dpkg-deb',
    `import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
const output = process.argv[process.argv.length - 1];
if (!output || output === '--build') {
  process.exit(2);
}
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, '');
`,
  );
  return binDir;
}

async function runPackageCommand(
  projectDir: string,
  options: { target?: string; format?: string },
): Promise<{
    exitCode: number | null;
    logs: string[];
    errors: string[];
  }> {
  const previousCwd = process.cwd();
  const logs: string[] = [];
  const errors: string[] = [];
  let exitCode: number | null = null;

  const logSpy = vi.spyOn(console, 'log').mockImplementation((...args: unknown[]) => {
    logs.push(args.map((arg) => String(arg)).join(' '));
  });
  const errorSpy = vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => {
    errors.push(args.map((arg) => String(arg)).join(' '));
  });
  const exitSpy = vi.spyOn(process, 'exit').mockImplementation(((code?: string | number | null) => {
    const normalized = code === undefined || code === null ? 0 : Number(code);
    exitCode = Number.isNaN(normalized) ? 1 : normalized;
    throw new Error(`__PROCESS_EXIT__${exitCode}`);
  }) as never);

  try {
    process.chdir(projectDir);
    await packageCommand(options);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (!message.startsWith('__PROCESS_EXIT__')) {
      throw error;
    }
  } finally {
    process.chdir(previousCwd);
    exitSpy.mockRestore();
    errorSpy.mockRestore();
    logSpy.mockRestore();
  }

  return { exitCode, logs, errors };
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    if (!dir) {
      continue;
    }
    rmSync(dir, { recursive: true, force: true });
  }
});

describe.runIf(process.platform === 'linux')('linux packaging CI filename checks', () => {
  it('emits AppImage and deb artifact filenames that match inferred architecture', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });

    const runtimePath = join(distDir, 'my-app');
    writeFileSync(runtimePath, 'binary', 'utf8');
    chmodSync(runtimePath, 0o755);
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'aarch64-unknown-linux-gnu',
    });

    const fakeToolDir = installFakeLinuxPackagingTools(projectDir);
    const previousPath = process.env.PATH ?? '';
    process.env.PATH = `${fakeToolDir}:${previousPath}`;

    try {
      const appImageResult = await runPackageCommand(projectDir, {
        target: 'aarch64-unknown-linux-gnu',
        format: 'appimage',
      });
      expect(appImageResult.exitCode).toBeNull();
      expect(existsSync(join(projectDir, 'dist-package', 'my-app-0.1.0-aarch64.AppImage'))).toBe(true);

      const debResult = await runPackageCommand(projectDir, {
        target: 'aarch64-unknown-linux-gnu',
        format: 'deb',
      });
      expect(debResult.exitCode).toBeNull();
      expect(existsSync(join(projectDir, 'dist-package', 'my-app_0.1.0_arm64.deb'))).toBe(true);
    } finally {
      process.env.PATH = previousPath;
    }
  });
});
