import type {
  AzureTrustedSigningConfig,
  DigiCertKeyLockerConfig,
  MacOSSigningConfig,
  WindowsSigningConfig,
} from 'voltkit';

/** Resolved macOS signing configuration (config + env vars merged). */
export interface ResolvedMacOSConfig {
  identity: string;
  entitlements?: string;
  notarize: boolean;
  teamId?: string;
  appleId?: string;
  applePassword?: string;
  /** Base64-encoded .p12 certificate (CI mode). */
  certificate?: string;
  /** Password for the .p12 certificate. */
  certificatePassword?: string;
}

/** Resolved Windows signing configuration (config + env vars merged). */
export type WindowsSigningProvider = 'local' | 'azureTrustedSigning' | 'digicertKeyLocker';

export interface ResolvedAzureTrustedSigningConfig extends AzureTrustedSigningConfig {
  dlibPath?: string;
  metadataPath?: string;
  endpoint?: string;
  accountName?: string;
  certificateProfileName?: string;
  correlationId?: string;
}

export interface ResolvedDigiCertKeyLockerConfig extends DigiCertKeyLockerConfig {
  keypairAlias?: string;
  certificateFingerprint?: string;
  smctlPath: string;
  timestampUrl?: string;
}

export interface ResolvedWindowsConfig {
  provider: WindowsSigningProvider;
  certificate?: string;
  certificatePassword?: string;
  digestAlgorithm: string;
  timestampUrl: string;
  azureTrustedSigning?: ResolvedAzureTrustedSigningConfig;
  digicertKeyLocker?: ResolvedDigiCertKeyLockerConfig;
}

/** Resolved signing configuration for all platforms. */
export interface ResolvedSigningConfig {
  macOS?: ResolvedMacOSConfig;
  windows?: ResolvedWindowsConfig;
}

export interface SigningArtifactResult {
  platform: 'darwin' | 'win32';
  provider: string;
  tool: string;
  targetPath: string;
  signed: boolean;
  notarized: boolean;
  startedAt: string;
  finishedAt: string;
  details?: Record<string, unknown>;
}

export interface PackageSigningConfigInput {
  signing?: {
    macOS?: MacOSSigningConfig;
    windows?: WindowsSigningConfig;
  };
}
