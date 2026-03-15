/**
 * The base directory for sandboxed file operations.
 * Defaults to process.cwd(). Set during app initialization.
 */
let baseDir = process.cwd();

export function getBaseDir(): string {
  return baseDir;
}

export function setBaseDir(dir: string): void {
  baseDir = dir;
}
