import { existsSync, mkdirSync, rmSync } from 'node:fs';
import type { MkdirPathFn, RemovePathFn } from './types.js';

export function cleanupAssetBundleIfExists(
  bundlePath: string | null,
  pathExists: (path: string) => boolean = existsSync,
  removePath: RemovePathFn = rmSync,
): boolean {
  if (!bundlePath || !pathExists(bundlePath)) {
    return false;
  }
  removePath(bundlePath, { force: true });
  return true;
}

export function cleanupDirectoryIfExists(
  dirPath: string | null,
  pathExists: (path: string) => boolean = existsSync,
  removePath: RemovePathFn = rmSync,
): boolean {
  if (!dirPath || !pathExists(dirPath)) {
    return false;
  }
  removePath(dirPath, { force: true, recursive: true });
  return true;
}

export function prepareOutputDirectory(
  outputDir: string,
  pathExists: (path: string) => boolean = existsSync,
  removePath: RemovePathFn = rmSync,
  makeDir: MkdirPathFn = mkdirSync,
): void {
  if (pathExists(outputDir)) {
    removePath(outputDir, { force: true, recursive: true });
  }
  makeDir(outputDir, { recursive: true });
}
