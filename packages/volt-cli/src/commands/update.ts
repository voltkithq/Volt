import { updatePublishCommand, runPreflightChecks } from './update/command.js';
import {
  buildPublishedArtifactRecord,
  buildUpdateReleaseManifest,
  sha256FileHex,
} from './update/manifest.js';
import { createPublishProvider } from './update/provider.js';

export { updatePublishCommand };
export type { UpdatePublishOptions } from './update/types.js';

export const __testOnly = {
  runPreflightChecks,
  buildPublishedArtifactRecord,
  buildUpdateReleaseManifest,
  createPublishProvider,
  sha256FileHex,
};
