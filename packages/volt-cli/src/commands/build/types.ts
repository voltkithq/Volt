export type BuildPlatform = 'win32' | 'darwin' | 'linux';
export type CargoArtifactKind = 'bin' | 'cdylib' | 'dylib' | 'staticlib';

export interface CargoMetadataTarget {
  name: string;
  kind: string[];
}

export interface CargoMetadataPackage {
  name: string;
  targets: CargoMetadataTarget[];
}

export interface CargoMetadata {
  workspace_root?: string;
  target_directory?: string;
  packages?: CargoMetadataPackage[];
}

export interface RuntimeArtifactCandidate {
  kind: CargoArtifactKind;
  targetName: string;
  fileName: string;
}

export interface ResolvedRuntimeArtifact {
  kind: CargoArtifactKind;
  targetName: string;
  sourcePath: string;
}

export type RemovePathFn = (
  path: string,
  options?: { force?: boolean; recursive?: boolean },
) => void;
export type MkdirPathFn = (path: string, options?: { recursive?: boolean }) => void;

export const ARTIFACT_KIND_PRIORITY: readonly CargoArtifactKind[] = [
  'bin',
  'cdylib',
  'dylib',
  'staticlib',
];
