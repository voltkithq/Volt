import type { PackagePlatform, RuntimeArtifactCompatibility, RuntimeArtifactDescriptor } from './types.js';

function normalizePlatform(platform: NodeJS.Platform): PackagePlatform {
  if (platform === 'win32') {
    return 'win32';
  }
  if (platform === 'darwin') {
    return 'darwin';
  }
  return 'linux';
}

export function normalizePackagePlatform(
  target: string | undefined,
  fallback: NodeJS.Platform = process.platform,
): PackagePlatform {
  if (!target || target.length === 0) {
    return normalizePlatform(fallback);
  }
  const normalized = target.toLowerCase();
  if (normalized.includes('windows') || normalized === 'win32') {
    return 'win32';
  }
  if (normalized.includes('darwin') || normalized.includes('apple') || normalized === 'macos') {
    return 'darwin';
  }
  if (normalized.includes('linux')) {
    return 'linux';
  }
  return normalizePlatform(fallback);
}

export function validateRuntimeArtifactCompatibility(
  artifact: RuntimeArtifactDescriptor,
  platform: PackagePlatform,
): RuntimeArtifactCompatibility {
  const extension = artifact.extension.toLowerCase();
  if (platform === 'win32') {
    if (artifact.runtimeKind !== 'executable' || extension !== '.exe') {
      return {
        ok: false,
        reason:
          `Windows packaging requires an executable runtime artifact (.exe), but found ${artifact.fileName}. `
          + 'Build a runnable executable target before packaging.',
      };
    }
    return { ok: true };
  }

  if (artifact.runtimeKind !== 'executable') {
    return {
      ok: false,
      reason:
        `${platform} packaging requires an executable runtime artifact, but found ${artifact.fileName}. `
        + 'Build a runnable executable target before packaging.',
    };
  }

  return { ok: true };
}
