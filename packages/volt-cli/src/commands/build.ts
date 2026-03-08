import { buildBackendBundle, buildRunnerConfigPayload, ensureBackendEntryWithinProject, ensureSupportedBackendExtension, resolveBackendEntry } from './build/backend.js';
import { buildCommand } from './build/command.js';
import { cleanupAssetBundleIfExists, cleanupDirectoryIfExists, prepareOutputDirectory } from './build/fs-utils.js';
import { artifactFileNameForTarget, collectRuntimeArtifactCandidates, inferBuildPlatform } from './build/platform.js';
import { fallbackRuntimeArtifactCandidates, selectRuntimeArtifact } from './build/runtime-artifact.js';
import { createScopedTempDirectory, recoverStaleScopedDirectories } from '../utils/temp-artifacts.js';

export { buildCommand };
export type { BuildOptions } from './build/command.js';

export const __testOnly = {
  inferBuildPlatform,
  artifactFileNameForTarget,
  collectRuntimeArtifactCandidates,
  fallbackRuntimeArtifactCandidates,
  selectRuntimeArtifact,
  cleanupAssetBundleIfExists,
  cleanupDirectoryIfExists,
  prepareOutputDirectory,
  ensureSupportedBackendExtension,
  ensureBackendEntryWithinProject,
  buildRunnerConfigPayload,
  resolveBackendEntry,
  buildBackendBundle,
  recoverStaleScopedDirectories,
  createScopedTempDirectory,
};
