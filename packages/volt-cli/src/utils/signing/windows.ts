import type { ResolvedWindowsConfig, SigningArtifactResult } from './types.js';

import { signWithAzureTrustedSigning } from './windows/azure.js';
import { signWithDigiCertKeyLocker } from './windows/digicert.js';
import { signWithLocalCertificate } from './windows/local.js';

export async function signWindows(
  exePath: string,
  config: ResolvedWindowsConfig,
): Promise<SigningArtifactResult> {
  const startedAt = new Date().toISOString();

  const result =
    config.provider === 'azureTrustedSigning'
      ? signWithAzureTrustedSigning(exePath, config)
      : config.provider === 'digicertKeyLocker'
        ? signWithDigiCertKeyLocker(exePath, config)
        : signWithLocalCertificate(exePath, config);

  return {
    ...result,
    startedAt,
    finishedAt: new Date().toISOString(),
  };
}
