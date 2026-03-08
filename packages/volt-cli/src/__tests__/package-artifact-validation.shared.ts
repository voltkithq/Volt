import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, vi } from 'vitest';
import { packageCommand } from '../commands/package.js';
import type { PackageOptions } from '../commands/package.js';

const tempDirs: string[] = [];

export function createTempDir(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
}

export function createTempDistDir(): string {
  return createTempDir('volt-package-artifacts-');
}

export function createTempProjectDir(): string {
  const dir = createTempDir('volt-package-project-');
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

export async function runPackageCommand(
  projectDir: string,
  options: PackageOptions,
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
