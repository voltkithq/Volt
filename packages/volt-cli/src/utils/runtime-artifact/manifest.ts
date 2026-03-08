import { basename, resolve } from 'node:path';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import type { BuildRuntimeArtifactManifest, CargoArtifactKind, RuntimeArtifactKind } from './types.js';

const RUNTIME_ARTIFACT_MANIFEST_FILE = '.volt-runtime-artifact.json';

function isSafeManifestArtifactFileName(fileName: string): boolean {
  if (fileName.length === 0 || fileName === '.' || fileName === '..') {
    return false;
  }
  if (fileName.includes('/') || fileName.includes('\\') || fileName.includes(':')) {
    return false;
  }
  return basename(fileName) === fileName;
}

function isCargoArtifactKind(kind: unknown): kind is CargoArtifactKind {
  return kind === 'bin' || kind === 'cdylib' || kind === 'dylib' || kind === 'staticlib';
}

export function runtimeKindFromCargoArtifactKind(kind: CargoArtifactKind): RuntimeArtifactKind {
  return kind === 'bin' ? 'executable' : 'library';
}

export function writeRuntimeArtifactManifest(
  outputDir: string,
  manifest: BuildRuntimeArtifactManifest,
): void {
  if (!isSafeManifestArtifactFileName(manifest.artifactFileName)) {
    throw new Error(
      `Invalid runtime artifact file name in manifest: "${manifest.artifactFileName}"`,
    );
  }
  const manifestPath = resolve(outputDir, RUNTIME_ARTIFACT_MANIFEST_FILE);
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf8');
}

export function readRuntimeArtifactManifest(outputDir: string): BuildRuntimeArtifactManifest | null {
  const manifestPath = resolve(outputDir, RUNTIME_ARTIFACT_MANIFEST_FILE);
  if (!existsSync(manifestPath)) {
    return null;
  }
  try {
    const parsed = JSON.parse(readFileSync(manifestPath, 'utf8')) as Partial<BuildRuntimeArtifactManifest>;
    if (parsed.schemaVersion !== 1) {
      return null;
    }
    if (typeof parsed.artifactFileName !== 'string' || parsed.artifactFileName.length === 0) {
      return null;
    }
    if (!isSafeManifestArtifactFileName(parsed.artifactFileName)) {
      return null;
    }
    if (!isCargoArtifactKind(parsed.cargoArtifactKind)) {
      return null;
    }
    if (typeof parsed.cargoTargetName !== 'string' || parsed.cargoTargetName.length === 0) {
      return null;
    }
    if (parsed.rustTarget !== null && parsed.rustTarget !== undefined && typeof parsed.rustTarget !== 'string') {
      return null;
    }

    return {
      schemaVersion: 1,
      artifactFileName: parsed.artifactFileName,
      cargoArtifactKind: parsed.cargoArtifactKind,
      cargoTargetName: parsed.cargoTargetName,
      rustTarget: parsed.rustTarget ?? null,
    };
  } catch {
    return null;
  }
}
