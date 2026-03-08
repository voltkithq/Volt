import type { CargoArtifactKind, CargoMetadataTarget, RuntimeArtifactCandidate, BuildPlatform } from './types.js';
import { ARTIFACT_KIND_PRIORITY } from './types.js';

export function inferBuildPlatform(target?: string, fallback: NodeJS.Platform = process.platform): BuildPlatform {
  if (!target) {
    return normalizePlatform(fallback);
  }

  const triple = target.toLowerCase();
  if (triple.includes('windows')) {
    return 'win32';
  }
  if (triple.includes('darwin') || triple.includes('apple')) {
    return 'darwin';
  }
  if (triple.includes('linux')) {
    return 'linux';
  }
  return normalizePlatform(fallback);
}

function normalizePlatform(platform: NodeJS.Platform): BuildPlatform {
  if (platform === 'win32') {
    return 'win32';
  }
  if (platform === 'darwin') {
    return 'darwin';
  }
  return 'linux';
}

export function artifactFileNameForTarget(
  targetName: string,
  kind: CargoArtifactKind,
  platform: BuildPlatform,
): string | null {
  switch (kind) {
    case 'bin':
      return platform === 'win32' ? `${targetName}.exe` : targetName;
    case 'cdylib':
    case 'dylib':
      if (platform === 'win32') {
        return `${targetName}.dll`;
      }
      if (platform === 'darwin') {
        return `lib${targetName}.dylib`;
      }
      return `lib${targetName}.so`;
    case 'staticlib':
      return platform === 'win32' ? `${targetName}.lib` : `lib${targetName}.a`;
    default:
      return null;
  }
}

export function collectRuntimeArtifactCandidates(
  targets: CargoMetadataTarget[],
  platform: BuildPlatform,
): RuntimeArtifactCandidate[] {
  const candidates: RuntimeArtifactCandidate[] = [];
  const seen = new Set<string>();
  for (const kind of ARTIFACT_KIND_PRIORITY) {
    for (const target of targets) {
      if (!target.kind.includes(kind)) {
        continue;
      }
      const fileName = artifactFileNameForTarget(target.name, kind, platform);
      if (!fileName) {
        continue;
      }
      const dedupeKey = `${kind}:${fileName}`;
      if (seen.has(dedupeKey)) {
        continue;
      }
      seen.add(dedupeKey);
      candidates.push({
        kind,
        targetName: target.name,
        fileName,
      });
    }
  }
  return candidates;
}
