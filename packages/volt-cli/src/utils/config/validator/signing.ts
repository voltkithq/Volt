import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import type { VoltConfig } from 'voltkit';
import { normalizeWindowsSigningProvider } from '../../signing/provider.js';
import { pushError, pushWarning, type ValidationContext } from './context.js';

const VALID_DIGEST_ALGORITHMS = ['sha256', 'sha384', 'sha512'];

export function validateSigningConfig(config: VoltConfig, context: ValidationContext): void {
  const signing = config.package?.signing;
  if (!signing) {
    return;
  }

  if (signing.macOS) {
    if (!signing.macOS.identity && !process.env['VOLT_MACOS_SIGNING_IDENTITY']) {
      pushWarning(
        context,
        `'package.signing.macOS.identity' is not set and VOLT_MACOS_SIGNING_IDENTITY env var is missing. macOS signing will be skipped.`,
      );
    }
    if (signing.macOS.entitlements && !existsSync(resolve(process.cwd(), signing.macOS.entitlements))) {
      pushError(
        context,
        `'package.signing.macOS.entitlements' path does not exist: ${signing.macOS.entitlements}`,
      );
    }
  }

  if (!signing.windows) {
    return;
  }

  const providerInput = String(
    signing.windows.provider ?? process.env['VOLT_WIN_SIGNING_PROVIDER'] ?? 'local',
  );
  let provider = normalizeWindowsSigningProvider(providerInput);

  if (provider === null) {
    pushError(
      context,
      `'package.signing.windows.provider' must be one of: local, azureTrustedSigning, digicertKeyLocker.`,
    );
    provider = 'local';
  }

  if (!signing.windows.certificate && !process.env['VOLT_WIN_CERTIFICATE'] && provider === 'local') {
    pushWarning(
      context,
      `'package.signing.windows.certificate' is not set and VOLT_WIN_CERTIFICATE env var is missing. Windows signing will be skipped.`,
    );
  }
  if (signing.windows.digestAlgorithm && !VALID_DIGEST_ALGORITHMS.includes(signing.windows.digestAlgorithm)) {
    pushError(
      context,
      `'package.signing.windows.digestAlgorithm' must be one of: ${VALID_DIGEST_ALGORITHMS.join(', ')}. Got: '${signing.windows.digestAlgorithm}'.`,
    );
    signing.windows.digestAlgorithm = undefined;
  }

  if (provider === 'azureTrustedSigning') {
    const dlibPath =
      signing.windows.azureTrustedSigning?.dlibPath
      ?? process.env['VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH'];
    const metadataPath =
      signing.windows.azureTrustedSigning?.metadataPath
      ?? process.env['VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH'];
    if (!dlibPath || !metadataPath) {
      pushWarning(
        context,
        'Azure Trusted Signing selected but dlib/metadata path is missing. Set package.signing.windows.azureTrustedSigning.{dlibPath,metadataPath} or VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH and VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH.',
      );
    }
  }

  if (provider === 'digicertKeyLocker') {
    const keypairAlias =
      signing.windows.digicertKeyLocker?.keypairAlias
      ?? process.env['VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS'];
    if (!keypairAlias) {
      pushWarning(
        context,
        'DigiCert KeyLocker selected but keypair alias is missing. Set package.signing.windows.digicertKeyLocker.keypairAlias or VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS.',
      );
    }
  }
}
