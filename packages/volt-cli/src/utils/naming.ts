const INVALID_NAME_ERROR = 'Application name must contain at least one alphanumeric character.';

/**
 * Convert a user-facing app name into a filesystem-safe binary/artifact stem.
 * The output is restricted to [a-z0-9._-] and never contains path separators.
 */
export function toSafeBinaryName(appName: string): string {
  const normalized = appName
    .toLowerCase()
    .trim()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9._-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^[._-]+|[._-]+$/g, '');

  if (!normalized || normalized === '.' || normalized === '..') {
    throw new Error(INVALID_NAME_ERROR);
  }

  return normalized;
}

/**
 * Normalize version-like strings for artifact filenames.
 * Keeps characters typically used in semver/build metadata.
 */
export function toSafeArtifactVersion(version: string): string {
  const normalized = version
    .trim()
    .replace(/[^0-9A-Za-z._+-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^[._+-]+|[._+-]+$/g, '');

  return normalized || '0.1.0';
}
