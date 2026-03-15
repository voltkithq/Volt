import { existsSync } from 'node:fs';

import { runSigningCommand } from '../command.js';
import { isToolAvailable } from '../tooling.js';
import type { ResolvedWindowsConfig } from '../types.js';
import type { SigningResultCore } from './types.js';
import { verifyWindowsSignature } from './verify.js';

export function signWithAzureTrustedSigning(
  exePath: string,
  config: ResolvedWindowsConfig,
): SigningResultCore {
  if (process.platform !== 'win32') {
    throw new Error('Azure Trusted Signing currently requires signtool on Windows.');
  }
  if (!isToolAvailable('signtool')) {
    throw new Error(
      'signtool is required for Azure Trusted Signing. Install Windows SDK and add signtool to PATH.',
    );
  }

  const azure = config.azureTrustedSigning;
  const dlibPath = azure?.dlibPath;
  const metadataPath = azure?.metadataPath;
  if (!dlibPath || !metadataPath) {
    throw new Error(
      'Azure Trusted Signing requires both dlibPath and metadataPath. Set package.signing.windows.azureTrustedSigning.{dlibPath,metadataPath} or VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH and VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH.',
    );
  }
  if (!existsSync(dlibPath)) {
    throw new Error(`Azure Trusted Signing dlib not found: ${dlibPath}`);
  }
  if (!existsSync(metadataPath)) {
    throw new Error(`Azure Trusted Signing metadata file not found: ${metadataPath}`);
  }

  console.log(`[volt] Signing ${exePath} with Azure Trusted Signing...`);
  const args = [
    'sign',
    '/fd',
    config.digestAlgorithm,
    '/tr',
    config.timestampUrl,
    '/td',
    config.digestAlgorithm,
    '/dlib',
    dlibPath,
    '/dmdf',
    metadataPath,
  ];
  if (azure?.correlationId) {
    args.push('/d', `VOLT_CORRELATION_ID=${azure.correlationId}`);
  }

  args.push(exePath);
  runSigningCommand('signtool', args, {
    description: 'signtool sign (Azure Trusted Signing)',
  });
  verifyWindowsSignature(exePath, 'signtool');
  console.log('[volt] Azure Trusted Signing complete.');

  return {
    platform: 'win32',
    provider: 'azureTrustedSigning',
    tool: 'signtool',
    targetPath: exePath,
    signed: true,
    notarized: false,
    details: {
      digestAlgorithm: config.digestAlgorithm,
      timestampUrl: config.timestampUrl,
      dlibPath,
      metadataPath,
      endpoint: azure?.endpoint,
      accountName: azure?.accountName,
      certificateProfileName: azure?.certificateProfileName,
      correlationId: azure?.correlationId,
    },
  };
}
