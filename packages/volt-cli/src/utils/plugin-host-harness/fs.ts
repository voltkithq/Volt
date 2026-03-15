import { mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve } from 'node:path';

export function performScopedFsRequest(
  baseDir: string,
  operation: string,
  payload: Record<string, unknown> | null,
): unknown {
  const path = requireString(payload, 'path');
  const resolved = safeResolve(baseDir, path);
  switch (operation) {
    case 'read-file':
      return readFileSync(resolved, 'utf8');
    case 'write-file':
      mkdirSync(dirname(resolved), { recursive: true });
      writeFileSync(resolved, requireString(payload, 'data'), 'utf8');
      return true;
    case 'read-dir':
      return readdirSync(resolved);
    case 'stat': {
      const info = statSync(resolved);
      return {
        size: info.size,
        isFile: info.isFile(),
        isDir: info.isDirectory(),
        readonly: false,
        modifiedMs: info.mtimeMs,
        createdMs: info.birthtimeMs || null,
      };
    }
    case 'exists':
      try {
        statSync(resolved);
        return true;
      } catch {
        return false;
      }
    case 'mkdir':
      mkdirSync(resolved, { recursive: true });
      return true;
    case 'remove':
      rmSync(resolved, { recursive: true, force: true });
      return true;
    default:
      throw new Error(`unsupported fs operation '${operation}'`);
  }
}

function requireString(payload: Record<string, unknown> | null, key: string): string {
  const value = payload?.[key];
  if (typeof value !== 'string' || value.trim().length === 0) {
    throw new Error(`payload is missing required '${key}' string`);
  }
  return value;
}

function safeResolve(baseDir: string, userPath: string): string {
  if (userPath.includes('\\') || isAbsolute(userPath)) {
    throw new Error(`path traversal is not allowed: ${userPath}`);
  }
  const resolved = resolve(baseDir, userPath);
  const relativePath = relative(resolve(baseDir), resolved);
  if (relativePath === '..' || relativePath.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`)) {
    throw new Error(`path escapes base directory: ${userPath}`);
  }
  return resolved;
}
