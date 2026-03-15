/**
 * Validate that a path is safe (no absolute paths, no traversal).
 * This is a TypeScript-side guard; the Rust side also validates.
 */
export function validatePath(path: string): void {
  if (path === '') {
    throw new Error('Path cannot be empty.');
  }

  if (path.includes('\0')) {
    throw new Error(`Null bytes are not allowed in paths: "${path}".`);
  }

  if (path.startsWith('/') || path.startsWith('\\') || /^[a-zA-Z]:/.test(path)) {
    throw new Error(
      `Absolute paths are not allowed: "${path}". Use paths relative to the app scope.`,
    );
  }

  const segments = path.split(/[\\/]+/).filter(Boolean);
  if (segments.includes('..')) {
    throw new Error(`Path traversal is not allowed: "${path}".`);
  }
}

/**
 * Validate a scoped path. Unlike validatePath, this allows empty strings
 * (to reference the scope root directory itself for readDir/stat/exists).
 */
export function validateScopedPath(path: string): void {
  if (path === '') {
    return;
  }

  validatePath(path);
}
