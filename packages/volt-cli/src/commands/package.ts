import { packageCommand } from './package/command.js';
import { validateRequestedPackageFormat } from './package/config.js';
import {
  escapeNsisString,
  escapeXml,
  inferAppImageArchitecture,
  inferDebArchitecture,
  isMissingExecutableError,
  normalizeMsixVersion,
  normalizeDebianControlVersion,
  runPackagingTool,
  runPackagingToolWithFallback,
} from './package/helpers.js';
import { generateAppRun, generateDesktopFile, generateMsixManifest, generateNsisScript } from './package/templates.js';
import {
  normalizePackagePlatform,
  resolveRuntimeArtifactForPackaging,
  validateRuntimeArtifactCompatibility,
} from '../utils/runtime-artifact.js';
import { __testOnly as packageConfigTestOnly } from './package/config.js';

export { packageCommand };
export type { PackageOptions } from './package/types.js';

export const __testOnly = {
  escapeNsisString,
  escapeXml,
  inferDebArchitecture,
  inferAppImageArchitecture,
  normalizeDebianControlVersion,
  normalizeMsixVersion,
  validateRequestedPackageFormat,
  runPackagingToolWithFallback,
  isMissingExecutableError,
  runPackagingTool,
  generateNsisScript,
  generateMsixManifest,
  generateAppRun,
  generateDesktopFile,
  normalizePackagePlatform,
  resolveRuntimeArtifactForPackaging,
  validateRuntimeArtifactCompatibility,
  normalizeWindowsInstallMode: packageConfigTestOnly.normalizeWindowsInstallMode,
};
