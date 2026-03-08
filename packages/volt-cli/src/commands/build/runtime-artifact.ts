import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { collectRuntimeArtifactCandidates, inferBuildPlatform } from './platform.js';
import type {
  BuildPlatform,
  CargoMetadata,
  CargoMetadataTarget,
  ResolvedRuntimeArtifact,
  RuntimeArtifactCandidate,
} from './types.js';

export function readCargoMetadata(cwd: string): CargoMetadata | null {
  try {
    const output = execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
      cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      encoding: 'utf-8',
    });
    return JSON.parse(output) as CargoMetadata;
  } catch {
    return null;
  }
}

export function fallbackRuntimeArtifactCandidates(platform: BuildPlatform): RuntimeArtifactCandidate[] {
  const fallbackTargets: CargoMetadataTarget[] = [
    { name: 'volt-runner', kind: ['bin'] },
    { name: 'volt_runner', kind: ['bin'] },
  ];
  return collectRuntimeArtifactCandidates(fallbackTargets, platform);
}

export function resolveRuntimeArtifact(
  releaseDir: string,
  target?: string,
  metadata?: CargoMetadata | null,
): {
  artifact: ResolvedRuntimeArtifact | null;
  attemptedPaths: string[];
} {
  const platform = inferBuildPlatform(target);
  const packageTargets =
    metadata?.packages?.find((pkg) => pkg.name === 'volt-runner')?.targets ?? [];

  const candidates =
    packageTargets.length > 0
      ? collectRuntimeArtifactCandidates(packageTargets, platform)
      : fallbackRuntimeArtifactCandidates(platform);
  return selectRuntimeArtifact(candidates, releaseDir);
}

export function selectRuntimeArtifact(
  candidates: RuntimeArtifactCandidate[],
  releaseDir: string,
  fileExists: (path: string) => boolean = existsSync,
): {
  artifact: ResolvedRuntimeArtifact | null;
  attemptedPaths: string[];
} {
  const attemptedPaths = candidates.map((candidate) => resolve(releaseDir, candidate.fileName));
  for (const candidate of candidates) {
    const sourcePath = resolve(releaseDir, candidate.fileName);
    if (!fileExists(sourcePath)) {
      continue;
    }
    return {
      artifact: {
        kind: candidate.kind,
        targetName: candidate.targetName,
        sourcePath,
      },
      attemptedPaths,
    };
  }
  return { artifact: null, attemptedPaths };
}
