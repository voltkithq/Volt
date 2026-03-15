import { existsSync, renameSync, unlinkSync } from 'node:fs';

import { runSigningCommand } from '../command.js';
import { isToolAvailable } from '../tooling.js';
import type { ResolvedWindowsConfig } from '../types.js';
import type { SigningResultCore } from './types.js';
import { verifyWindowsSignature } from './verify.js';

export function signWithLocalCertificate(
  exePath: string,
  config: ResolvedWindowsConfig,
): SigningResultCore {
  const certificate = config.certificate;
  if (!certificate) {
    throw new Error(
      'Windows signing provider "local" requires a certificate path. Set package.signing.windows.certificate or VOLT_WIN_CERTIFICATE.',
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
    runSigningCommand('signtool', args, { description: 'signtool sign' });
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

  runSigningCommand('osslsigncode', args, { description: 'osslsigncode sign' });
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
