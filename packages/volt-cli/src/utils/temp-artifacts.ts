import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

export interface StaleDirectoryRecoveryOptions {
  prefix: string;
  staleAfterMs: number;
  nowMs?: number;
}

export interface StaleDirectoryRecoveryResult {
  scanned: number;
  removed: number;
  failures: number;
}

export function createScopedTempDirectory(rootDir: string, prefix: string): string {
  mkdirSync(rootDir, { recursive: true });
  return mkdtempSync(resolve(rootDir, prefix));
}

export function recoverStaleScopedDirectories(
  rootDir: string,
  options: StaleDirectoryRecoveryOptions,
): StaleDirectoryRecoveryResult {
  if (!existsSync(rootDir)) {
    return {
      scanned: 0,
      removed: 0,
      failures: 0,
    };
  }

  const nowMs = options.nowMs ?? Date.now();
  const entries = readdirSync(rootDir, { withFileTypes: true });
  let scanned = 0;
  let removed = 0;
  let failures = 0;

  for (const entry of entries) {
    if (!entry.isDirectory() || !entry.name.startsWith(options.prefix)) {
      continue;
    }
    scanned += 1;

    const absolutePath = join(rootDir, entry.name);
    try {
      const stats = statSync(absolutePath);
      const ageMs = nowMs - stats.mtimeMs;
      if (ageMs < options.staleAfterMs) {
        continue;
      }
      rmSync(absolutePath, { recursive: true, force: true });
      removed += 1;
    } catch {
      failures += 1;
    }
  }

  return {
    scanned,
    removed,
    failures,
  };
}
