import { basename, join, resolve } from 'node:path';
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';
import type { VoltTestLogger } from './types.js';

const EXCLUDED_COPY_DIRS = new Set([
  'dist',
  'dist-volt',
  '.turbo',
  'target',
  'coverage',
]);

export interface CopiedProjectPaths {
  tempRoot: string;
  tempProjectDir: string;
}

export function copyProjectToTemp(projectDir: string, repoRoot: string): CopiedProjectPaths {
  const resolvedSource = resolve(projectDir);
  if (!existsSync(resolvedSource)) {
    throw new Error(`[volt:test] source project does not exist: ${resolvedSource}`);
  }

  const tempRoot = mkdtempSync(join(resolve(repoRoot), '.volt-test-'));
  const tempProjectDir = join(tempRoot, basename(resolvedSource));

  cpSync(resolvedSource, tempProjectDir, {
    recursive: true,
    force: true,
    filter: (sourcePath) => {
      const name = basename(sourcePath);
      return !EXCLUDED_COPY_DIRS.has(name);
    },
  });

  return { tempRoot, tempProjectDir };
}

export async function cleanupDirectoryBestEffort(directoryPath: string, logger: VoltTestLogger): Promise<void> {
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      rmSync(directoryPath, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 7) {
        logger.warn(
          `[volt:test] cleanup warning for ${directoryPath}: ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
        return;
      }
      await delay(250);
    }
  }
}

export function readTextFile(filePath: string): string {
  return readFileSync(filePath, 'utf8');
}

export function writeTextFile(filePath: string, contents: string): void {
  writeFileSync(filePath, contents, 'utf8');
}
