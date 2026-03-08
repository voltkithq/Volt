export type CargoArtifactKind = 'bin' | 'cdylib' | 'dylib' | 'staticlib';
export type RuntimeArtifactKind = 'executable' | 'library';
export type PackagePlatform = 'win32' | 'darwin' | 'linux';

export interface BuildRuntimeArtifactManifest {
  schemaVersion: 1;
  artifactFileName: string;
  cargoArtifactKind: CargoArtifactKind;
  cargoTargetName: string;
  rustTarget: string | null;
}

export interface RuntimeArtifactResolution {
  artifact: RuntimeArtifactDescriptor | null;
  attemptedPaths: string[];
}

export interface RuntimeArtifactDescriptor {
  fileName: string;
  absolutePath: string;
  extension: string;
  runtimeKind: RuntimeArtifactKind;
  cargoArtifactKind: CargoArtifactKind | null;
  rustTarget: string | null;
}

export interface RuntimeArtifactCompatibility {
  ok: boolean;
  reason?: string;
}
