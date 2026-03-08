import { resolveWindowsSigningProvider } from './provider.js';
import type { PackageSigningConfigInput, ResolvedSigningConfig } from './types.js';

function isMacPlatform(platform: string): boolean {
  return platform === 'darwin' || platform.includes('apple') || platform.includes('macos');
}

function isWindowsPlatform(platform: string): boolean {
  return platform === 'win32' || platform.includes('windows');
}

function readOptional(value: string | undefined): string | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }
  const normalized = value.trim();
  return normalized.length > 0 ? normalized : undefined;
}

/**
 * Merge config file values with environment variable overrides.
 * Returns null if no signing is configured for the target platform.
 * Environment variables always take precedence over config file values.
 */
export function resolveSigningConfig(
  packageConfig: PackageSigningConfigInput | undefined,
  platform: string,
): ResolvedSigningConfig | null {
  const signing = packageConfig?.signing;

  if (isMacPlatform(platform)) {
    const macConfig = signing?.macOS;
    const identity = process.env['VOLT_MACOS_SIGNING_IDENTITY'] ?? macConfig?.identity;
    if (!identity) {
      return null;
    }

    return {
      macOS: {
        identity,
        entitlements: macConfig?.entitlements,
        notarize: macConfig?.notarize ?? true,
        teamId: process.env['VOLT_APPLE_TEAM_ID'] ?? macConfig?.teamId,
        appleId: process.env['VOLT_APPLE_ID'],
        applePassword: process.env['VOLT_APPLE_PASSWORD'],
        certificate: process.env['VOLT_MACOS_CERTIFICATE'],
        certificatePassword: process.env['VOLT_MACOS_CERTIFICATE_PASSWORD'],
      },
    };
  }

  if (isWindowsPlatform(platform)) {
    const winConfig = signing?.windows;
    const provider = resolveWindowsSigningProvider(
      process.env['VOLT_WIN_SIGNING_PROVIDER'] ?? winConfig?.provider,
    );

    const common = {
      provider,
      digestAlgorithm: winConfig?.digestAlgorithm ?? 'sha256',
      timestampUrl:
        process.env['VOLT_WIN_TIMESTAMP_URL']
        ?? winConfig?.timestampUrl
        ?? 'https://timestamp.digicert.com',
    } as const;

    if (provider === 'local') {
      const certificate = process.env['VOLT_WIN_CERTIFICATE'] ?? winConfig?.certificate;
      if (!certificate) {
        return null;
      }

      return {
        windows: {
          ...common,
          certificate,
          certificatePassword: process.env['VOLT_WIN_CERTIFICATE_PASSWORD'],
        },
      };
    }

    if (provider === 'azureTrustedSigning') {
      const azureConfig = winConfig?.azureTrustedSigning;
      return {
        windows: {
          ...common,
          azureTrustedSigning: {
            dlibPath:
              process.env['VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH']
              ?? azureConfig?.dlibPath,
            metadataPath:
              process.env['VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH']
              ?? azureConfig?.metadataPath,
            endpoint:
              readOptional(process.env['VOLT_AZURE_TRUSTED_SIGNING_ENDPOINT'])
              ?? azureConfig?.endpoint,
            accountName:
              readOptional(process.env['VOLT_AZURE_TRUSTED_SIGNING_ACCOUNT_NAME'])
              ?? azureConfig?.accountName,
            certificateProfileName:
              readOptional(process.env['VOLT_AZURE_TRUSTED_SIGNING_CERT_PROFILE'])
              ?? azureConfig?.certificateProfileName,
            correlationId:
              readOptional(process.env['VOLT_AZURE_TRUSTED_SIGNING_CORRELATION_ID'])
              ?? azureConfig?.correlationId,
          },
        },
      };
    }

    const digicertConfig = winConfig?.digicertKeyLocker;
    return {
      windows: {
        ...common,
        digicertKeyLocker: {
          keypairAlias:
            process.env['VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS']
            ?? digicertConfig?.keypairAlias,
          certificateFingerprint:
            process.env['VOLT_DIGICERT_KEYLOCKER_CERT_FINGERPRINT']
            ?? digicertConfig?.certificateFingerprint,
          smctlPath:
            readOptional(process.env['VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH'])
            ?? digicertConfig?.smctlPath
            ?? 'smctl',
          timestampUrl:
            readOptional(process.env['VOLT_DIGICERT_KEYLOCKER_TIMESTAMP_URL'])
            ?? digicertConfig?.timestampUrl,
        },
      },
    };
  }

  // Linux: no signing needed
  return null;
}
