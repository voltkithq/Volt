import { existsSync } from 'node:fs';

import { runSigningCommand } from '../command.js';
import { isToolAvailable } from '../tooling.js';
import type { ResolvedWindowsConfig } from '../types.js';
import type { SigningResultCore } from './types.js';
import { verifyWindowsSignature } from './verify.js';

export function signWithDigiCertKeyLocker(
  exePath: string,
  config: ResolvedWindowsConfig,
): SigningResultCore {
  const digicert = config.digicertKeyLocker;
  const smctlPath = digicert?.smctlPath ?? 'smctl';
  const keypairAlias = digicert?.keypairAlias;
  if (!keypairAlias) {
    throw new Error(
      'DigiCert KeyLocker signing requires keypairAlias. Set package.signing.windows.digicertKeyLocker.keypairAlias or VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS.',
    );
  }

  const toolAvailable =
    smctlPath.includes('/') || smctlPath.includes('\\')
      ? existsSync(smctlPath)
      : isToolAvailable(smctlPath);
  if (!toolAvailable) {
    throw new Error(
      `DigiCert KeyLocker tool not found: ${smctlPath}. Install smctl or set VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH.`,
    );
  }

  console.log(`[volt] Signing ${exePath} with DigiCert KeyLocker...`);
  const args = [
    'sign',
    '--input',
    exePath,
    '--keypair-alias',
    keypairAlias,
    '--digest-algorithm',
    config.digestAlgorithm,
    '--timestamp-url',
    digicert?.timestampUrl ?? config.timestampUrl,
  ];
  if (digicert?.certificateFingerprint) {
    args.push('--certificate-fingerprint', digicert.certificateFingerprint);
  }

  runSigningCommand(smctlPath, args, { description: 'smctl sign' });
  verifyWindowsSignature(exePath);
  console.log('[volt] DigiCert KeyLocker signing complete.');

  return {
    platform: 'win32',
    provider: 'digicertKeyLocker',
    tool: smctlPath,
    targetPath: exePath,
    signed: true,
    notarized: false,
    details: {
      digestAlgorithm: config.digestAlgorithm,
      timestampUrl: digicert?.timestampUrl ?? config.timestampUrl,
      keypairAlias,
      certificateFingerprint: digicert?.certificateFingerprint,
    },
  };
}
