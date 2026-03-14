import type { MacOSSigningConfig, WindowsSigningConfig } from 'voltkit';

export interface PackageOptions {
  target?: string;
  format?: string;
  installMode?: string;
  json?: boolean;
  jsonOutput?: string;
  /** @internal Skip pre-flight validation (testing only). */
  _skipPreflight?: boolean;
}

export type ExecFileSyncFn = (
  command: string,
  args: readonly string[],
  options?: { stdio?: 'inherit' | 'pipe' | readonly unknown[] },
) => unknown;

/** Package configuration from volt.config.ts. */
export interface PackageConfig {
  identifier: string;
  icon?: string;
  categories?: string[];
  windows?: PackageWindowsConfig;
  enterprise?: PackageEnterpriseConfig;
  signing?: {
    macOS?: MacOSSigningConfig;
    windows?: WindowsSigningConfig;
  };
}

export type WindowsInstallMode = 'perMachine' | 'perUser';

export interface PackageWindowsMsixConfig {
  identityName?: string;
  publisher?: string;
  publisherDisplayName?: string;
  displayName?: string;
  description?: string;
}

export interface PackageWindowsConfig {
  installMode?: WindowsInstallMode;
  silentAllUsers?: boolean;
  msix?: PackageWindowsMsixConfig;
}

export interface PackageEnterpriseConfig {
  generateAdmx?: boolean;
  includeDocsBundle?: boolean;
}

export const ALLOWED_PACKAGE_FORMATS: Readonly<Record<'win32' | 'darwin' | 'linux', readonly string[]>> = {
  win32: ['nsis', 'msix'],
  darwin: ['app', 'dmg'],
  linux: ['appimage', 'deb'],
};

export const WINDOWS_UPDATER_HELPER_FILE_NAME = 'volt-updater-helper.exe';

export interface PackageArtifactSummary {
  path: string;
  fileName: string;
}

export interface PackageCommandSummary {
  appName: string;
  version: string;
  platform: 'win32' | 'darwin' | 'linux';
  format: string | null;
  installMode: WindowsInstallMode | null;
  identifier: string;
  runtimeArtifact: string;
  outputDir: string;
  startedAt: string;
  finishedAt: string;
  durationMs: number;
  codeSigningEnabled: boolean;
  signingResults: import('../../utils/signing.js').SigningArtifactResult[];
  artifacts: PackageArtifactSummary[];
}
