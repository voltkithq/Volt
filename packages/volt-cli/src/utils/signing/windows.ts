import { existsSync, renameSync, unlinkSync } from 'node:fs';
import { runSigningCommand } from './command.js';
import { isToolAvailable } from './tooling.js';
import type { ResolvedWindowsConfig, SigningArtifactResult } from './types.js';

type VerificationTool = 'osslsigncode' | 'signtool';

/**
 * Sign a Windows executable using one of the configured providers.
 */
export async function signWindows(
  exePath: string,
  config: ResolvedWindowsConfig,
): Promise<SigningArtifactResult> {
  const startedAt = new Date().toISOString();

  switch (config.provider) {
    case 'azureTrustedSigning': {
      const result = signWithAzureTrustedSigning(exePath, config);
      return {
        ...result,
        startedAt,
        finishedAt: new Date().toISOString(),
      };
    }
    case 'digicertKeyLocker': {
      const result = signWithDigiCertKeyLocker(exePath, config);
      return {
        ...result,
        startedAt,
        finishedAt: new Date().toISOString(),
      };
    }
    case 'local':
    default: {
      const result = signWithLocalCertificate(exePath, config);
      return {
        ...result,
        startedAt,
        finishedAt: new Date().toISOString(),
      };
    }
  }
}

function selectVerificationTool(): VerificationTool | null {
  if (process.platform === 'win32' && isToolAvailable('signtool')) {
    return 'signtool';
  }
  if (isToolAvailable('osslsigncode')) {
    return 'osslsigncode';
  }
  if (isToolAvailable('signtool')) {
    return 'signtool';
  }
  return null;
}

function verifyWindowsSignature(exePath: string, preferredTool?: VerificationTool): void {
  const tool = preferredTool ?? selectVerificationTool();
  if (!tool) {
    throw new Error(
      'Signature verification requires signtool.exe or osslsigncode. '
        + 'Install one of them and ensure it is on PATH.',
    );
  }

  console.log('[volt] Verifying signature...');
  if (tool === 'signtool') {
    runSigningCommand('signtool', ['verify', '/pa', exePath], {
      description: 'signtool verify',
    });
    return;
  }

  runSigningCommand('osslsigncode', ['verify', exePath], {
    description: 'osslsigncode verify',
  });
}

function signWithLocalCertificate(
  exePath: string,
  config: ResolvedWindowsConfig,
): Omit<SigningArtifactResult, 'startedAt' | 'finishedAt'> {
  const certificate = config.certificate;
  if (!certificate) {
    throw new Error(
      'Windows signing provider "local" requires a certificate path. '
        + 'Set package.signing.windows.certificate or VOLT_WIN_CERTIFICATE.',
    );
  }

  if (!existsSync(certificate)) {
    throw new Error(`Certificate file not found: ${certificate}`);
  }

  const useSigntool = process.platform === 'win32' && isToolAvailable('signtool');
  const useOsslsigncode = !useSigntool && isToolAvailable('osslsigncode');
  if (!useSigntool && !useOsslsigncode) {
    throw new Error(
      'No signing tool found. Install signtool.exe (Windows SDK) or osslsigncode (cross-platform).',
    );
  }

  console.log(`[volt] Signing ${exePath} with local certificate...`);

  if (useSigntool) {
    const args = [
      'sign',
      '/f',
      certificate,
      '/tr',
      config.timestampUrl,
      '/td',
      config.digestAlgorithm,
      '/fd',
      config.digestAlgorithm,
    ];
    if (config.certificatePassword) {
      args.push('/p', config.certificatePassword);
    }
    args.push(exePath);
    runSigningCommand('signtool', args, {
      description: 'signtool sign',
    });
    verifyWindowsSignature(exePath, 'signtool');

    console.log('[volt] Windows signing complete.');
    return {
      platform: 'win32',
      provider: 'local',
      tool: 'signtool',
      targetPath: exePath,
      signed: true,
      notarized: false,
      details: {
        digestAlgorithm: config.digestAlgorithm,
        timestampUrl: config.timestampUrl,
      },
    };
  }

  const signedPath = `${exePath}.signed`;
  const args = [
    'sign',
    '-pkcs12',
    certificate,
    '-h',
    config.digestAlgorithm,
    '-t',
    config.timestampUrl,
    '-in',
    exePath,
    '-out',
    signedPath,
  ];
  if (config.certificatePassword) {
    args.push('-pass', config.certificatePassword);
  }

  runSigningCommand('osslsigncode', args, {
    description: 'osslsigncode sign',
  });
  unlinkSync(exePath);
  renameSync(signedPath, exePath);
  verifyWindowsSignature(exePath, 'osslsigncode');

  console.log('[volt] Windows signing complete.');
  return {
    platform: 'win32',
    provider: 'local',
    tool: 'osslsigncode',
    targetPath: exePath,
    signed: true,
    notarized: false,
    details: {
      digestAlgorithm: config.digestAlgorithm,
      timestampUrl: config.timestampUrl,
    },
  };
}

function signWithAzureTrustedSigning(
  exePath: string,
  config: ResolvedWindowsConfig,
): Omit<SigningArtifactResult, 'startedAt' | 'finishedAt'> {
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
      'Azure Trusted Signing requires both dlibPath and metadataPath. '
        + 'Set package.signing.windows.azureTrustedSigning.{dlibPath,metadataPath} '
        + 'or VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH and VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH.',
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

function signWithDigiCertKeyLocker(
  exePath: string,
  config: ResolvedWindowsConfig,
): Omit<SigningArtifactResult, 'startedAt' | 'finishedAt'> {
  const digicert = config.digicertKeyLocker;
  const smctlPath = digicert?.smctlPath ?? 'smctl';
  const keypairAlias = digicert?.keypairAlias;
  if (!keypairAlias) {
    throw new Error(
      'DigiCert KeyLocker signing requires keypairAlias. '
        + 'Set package.signing.windows.digicertKeyLocker.keypairAlias '
        + 'or VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS.',
    );
  }

  const toolAvailable = smctlPath.includes('/') || smctlPath.includes('\\')
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

  runSigningCommand(smctlPath, args, {
    description: 'smctl sign',
  });
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
