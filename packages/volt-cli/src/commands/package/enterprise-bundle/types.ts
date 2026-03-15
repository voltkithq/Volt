import type { VoltConfig } from 'voltkit';

import type { PackageArtifactSummary, PackageConfig, WindowsInstallMode } from '../types.js';

export interface EnterpriseBundleOptions {
  appName: string;
  version: string;
  packageDir: string;
  packageConfig: PackageConfig;
  config: VoltConfig;
  installMode: WindowsInstallMode | null;
  artifacts: readonly PackageArtifactSummary[];
}

export interface EnterpriseBundleResult {
  bundleDir: string;
  generatedFiles: string[];
}
