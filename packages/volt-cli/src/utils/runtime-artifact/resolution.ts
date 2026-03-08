import { existsSync, readdirSync } from 'node:fs';
import { extname, isAbsolute, relative, resolve } from 'node:path';
import type { RuntimeArtifactKind, RuntimeArtifactResolution } from './types.js';
import { readRuntimeArtifactManifest, runtimeKindFromCargoArtifactKind } from './manifest.js';

function isPathWithinDirectory(directoryPath: string, candidatePath: string): boolean {
  const relativePath = relative(directoryPath, candidatePath);
  if (relativePath.length === 0) {
    return false;
  }
  return !relativePath.startsWith('..') && !isAbsolute(relativePath);
}

function runtimeKindFromFileName(fileName: string): RuntimeArtifactKind {
  const lower = fileName.toLowerCase();
  const extension = extname(lower);
  if (extension === '.exe' || extension.length === 0) {
    return 'executable';
  }
  return 'library';
}

function packagingCandidateFileNames(binaryName: string): string[] {
  return [
    `${binaryName}.exe`,
    binaryName,
    `${binaryName}.dll`,
    `${binaryName}.dylib`,
    `${binaryName}.so`,
    `lib${binaryName}.so`,
    `lib${binaryName}.dylib`,
    `lib${binaryName}.a`,
    `${binaryName}.lib`,
  ];
}

export function resolveRuntimeArtifactForPackaging(
  distDir: string,
  binaryName: string,
): RuntimeArtifactResolution {
  const manifest = readRuntimeArtifactManifest(distDir);
  if (manifest) {
    const fromManifestPath = resolve(distDir, manifest.artifactFileName);
    if (isPathWithinDirectory(distDir, fromManifestPath) && existsSync(fromManifestPath)) {
      const fileNameRuntimeKind = runtimeKindFromFileName(manifest.artifactFileName);
      const manifestRuntimeKind = runtimeKindFromCargoArtifactKind(manifest.cargoArtifactKind);
      const effectiveRuntimeKind =
        fileNameRuntimeKind === 'executable' && manifestRuntimeKind === 'executable'
          ? 'executable'
          : 'library';
      return {
        artifact: {
          fileName: manifest.artifactFileName,
          absolutePath: fromManifestPath,
          extension: extname(manifest.artifactFileName),
          runtimeKind: effectiveRuntimeKind,
          cargoArtifactKind: manifest.cargoArtifactKind,
          rustTarget: manifest.rustTarget,
        },
        attemptedPaths: [fromManifestPath],
      };
    }
  }

  const candidatePaths = packagingCandidateFileNames(binaryName).map((fileName) => resolve(distDir, fileName));
  for (const candidatePath of candidatePaths) {
    if (!existsSync(candidatePath)) {
      continue;
    }
    const fileName = candidatePath.slice(distDir.length + 1);
    return {
      artifact: {
        fileName,
        absolutePath: candidatePath,
        extension: extname(fileName),
        runtimeKind: runtimeKindFromFileName(fileName),
        cargoArtifactKind: null,
        rustTarget: null,
      },
      attemptedPaths: candidatePaths,
    };
  }

  const fallbackMatches = readdirSync(distDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.startsWith(`${binaryName}.`))
    .map((entry) => entry.name)
    .sort();
  for (const fileName of fallbackMatches) {
    const candidatePath = resolve(distDir, fileName);
    if (!existsSync(candidatePath)) {
      continue;
    }
    return {
      artifact: {
        fileName,
        absolutePath: candidatePath,
        extension: extname(fileName),
        runtimeKind: runtimeKindFromFileName(fileName),
        cargoArtifactKind: null,
        rustTarget: null,
      },
      attemptedPaths: candidatePaths,
    };
  }

  return { artifact: null, attemptedPaths: candidatePaths };
}
