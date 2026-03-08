export type {
  PackageSigningConfigInput,
  ResolvedAzureTrustedSigningConfig,
  ResolvedDigiCertKeyLockerConfig,
  ResolvedMacOSConfig,
  ResolvedSigningConfig,
  SigningArtifactResult,
  ResolvedWindowsConfig,
  WindowsSigningProvider,
} from './signing/types.js';
export { resolveSigningConfig } from './signing/config.js';
export { isToolAvailable } from './signing/tooling.js';
export { cleanupKeychain, importCertificateToKeychain, signMacOS } from './signing/mac.js';
export { signWindows } from './signing/windows.js';
