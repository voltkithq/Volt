import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import type { VoltConfig } from 'voltkit';
import { DEFAULT_CONFIG, VALID_PERMISSIONS } from './constants.js';
import type { LoadConfigOptions } from './types.js';
import { normalizeWindowsSigningProvider } from '../signing/provider.js';

const BASE64_ED25519_PUBLIC_KEY_LENGTH = 32;
const LOOPBACK_IPV4_OCTET_COUNT = 4;

export function validateConfig(config: VoltConfig, filename: string, options: LoadConfigOptions): VoltConfig {
  const errors: string[] = [];
  if (!config.name || typeof config.name !== 'string') {
    const message = `'name' must be a non-empty string.`;
    console.error(`[volt] Error in ${filename}: ${message}`);
    console.error(`[volt] Example: defineConfig({ name: 'My App', ... })`);
    errors.push(message);
    config.name = DEFAULT_CONFIG.name;
  }

  if (config.version !== undefined && typeof config.version !== 'string') {
    const message = `'version' must be a string (e.g., "1.0.0").`;
    console.error(`[volt] Error in ${filename}: ${message}`);
    errors.push(message);
    config.version = undefined;
  }

  const configRecord = config as unknown as Record<string, unknown>;
  if (configRecord.backend !== undefined) {
    if (typeof configRecord.backend !== 'string' || configRecord.backend.trim().length === 0) {
      const message = `'backend' must be a non-empty string path (e.g., "./src/backend.ts").`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      delete configRecord.backend;
    } else {
      configRecord.backend = configRecord.backend.trim();
    }
  }

  if (config.window) {
    const w = config.window;
    if (w.width !== undefined && (typeof w.width !== 'number' || w.width <= 0)) {
      const message = `'window.width' must be a positive number.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      w.width = DEFAULT_CONFIG.window?.width;
    }
    if (w.height !== undefined && (typeof w.height !== 'number' || w.height <= 0)) {
      const message = `'window.height' must be a positive number.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      w.height = DEFAULT_CONFIG.window?.height;
    }
  }

  if (config.permissions) {
    if (!Array.isArray(config.permissions)) {
      const message = `'permissions' must be an array.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      console.error(`[volt] Valid permissions: ${VALID_PERMISSIONS.join(', ')}`);
      errors.push(message);
      config.permissions = [];
    } else {
      const filtered = config.permissions.filter((perm): perm is (typeof VALID_PERMISSIONS)[number] => {
        const valid = VALID_PERMISSIONS.includes(perm);
        if (!valid) {
          const message = `Unknown permission '${perm}'.`;
          console.error(`[volt] Error in ${filename}: ${message}`);
          console.error(`[volt] Valid permissions: ${VALID_PERMISSIONS.join(', ')}`);
          errors.push(message);
        }
        return valid;
      });
      config.permissions = filtered;
    }
  }

  if (config.updater) {
    const updater = config.updater;
    let updaterValid = true;
    if (!updater.endpoint || typeof updater.endpoint !== 'string' || !isValidUpdaterEndpoint(updater.endpoint)) {
      const message =
        `'updater.endpoint' must be an HTTPS URL or an HTTP localhost/loopback URL for local testing.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      updaterValid = false;
    }
    if (!updater.publicKey || typeof updater.publicKey !== 'string' || !isValidEd25519PublicKey(updater.publicKey)) {
      const message = `'updater.publicKey' must be a base64 Ed25519 public key.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      updaterValid = false;
    }
    if (!updaterValid) {
      config.updater = undefined;
    }
  }

  const runtime = configRecord.runtime as Record<string, unknown> | undefined;
  if (runtime !== undefined) {
    const poolSize = runtime.poolSize;
    if (
      poolSize !== undefined
      && (typeof poolSize !== 'number' || !Number.isInteger(poolSize) || poolSize <= 0)
    ) {
      const message = `'runtime.poolSize' must be a positive integer.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      delete runtime.poolSize;
    }
  }

  const runtimePoolSizeLegacy = configRecord.runtimePoolSize;
  if (
    runtimePoolSizeLegacy !== undefined
    && (typeof runtimePoolSizeLegacy !== 'number'
      || !Number.isInteger(runtimePoolSizeLegacy)
      || runtimePoolSizeLegacy <= 0)
  ) {
    const message = `'runtimePoolSize' must be a positive integer.`;
    console.error(`[volt] Error in ${filename}: ${message}`);
    errors.push(message);
    delete configRecord.runtimePoolSize;
  }

  const packageRecord = configRecord.package as Record<string, unknown> | undefined;
  if (packageRecord) {
    const windowsRecord = packageRecord.windows as Record<string, unknown> | undefined;
    if (windowsRecord !== undefined && typeof windowsRecord !== 'object') {
      const message = `'package.windows' must be an object when provided.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      delete packageRecord.windows;
    } else if (windowsRecord) {
      const installMode = windowsRecord.installMode;
      if (installMode !== undefined) {
        const normalizedInstallMode = normalizeWindowsInstallMode(installMode);
        if (!normalizedInstallMode) {
          const message = `'package.windows.installMode' must be "perMachine" or "perUser".`;
          console.error(`[volt] Error in ${filename}: ${message}`);
          errors.push(message);
          delete windowsRecord.installMode;
        } else {
          windowsRecord.installMode = normalizedInstallMode;
        }
      }

      const silentAllUsers = windowsRecord.silentAllUsers;
      if (silentAllUsers !== undefined && typeof silentAllUsers !== 'boolean') {
        const message = `'package.windows.silentAllUsers' must be a boolean when provided.`;
        console.error(`[volt] Error in ${filename}: ${message}`);
        errors.push(message);
        delete windowsRecord.silentAllUsers;
      }

      const msixRecord = windowsRecord.msix as Record<string, unknown> | undefined;
      if (msixRecord !== undefined && typeof msixRecord !== 'object') {
        const message = `'package.windows.msix' must be an object when provided.`;
        console.error(`[volt] Error in ${filename}: ${message}`);
        errors.push(message);
        delete windowsRecord.msix;
      } else if (msixRecord) {
        for (const field of ['identityName', 'publisher', 'publisherDisplayName', 'displayName', 'description']) {
          const value = msixRecord[field];
          if (value !== undefined && typeof value !== 'string') {
            const message = `'package.windows.msix.${field}' must be a string when provided.`;
            console.error(`[volt] Error in ${filename}: ${message}`);
            errors.push(message);
            delete msixRecord[field];
          }
        }
      }
    }

    const enterpriseRecord = packageRecord.enterprise as Record<string, unknown> | undefined;
    if (enterpriseRecord !== undefined && typeof enterpriseRecord !== 'object') {
      const message = `'package.enterprise' must be an object when provided.`;
      console.error(`[volt] Error in ${filename}: ${message}`);
      errors.push(message);
      delete packageRecord.enterprise;
    } else if (enterpriseRecord) {
      for (const field of ['generateAdmx', 'includeDocsBundle']) {
        const value = enterpriseRecord[field];
        if (value !== undefined && typeof value !== 'boolean') {
          const message = `'package.enterprise.${field}' must be a boolean when provided.`;
          console.error(`[volt] Error in ${filename}: ${message}`);
          errors.push(message);
          delete enterpriseRecord[field];
        }
      }
    }
  }

  const signing = config.package?.signing;
  if (signing) {
    const VALID_DIGEST_ALGORITHMS = ['sha256', 'sha384', 'sha512'];

    if (signing.macOS) {
      if (!signing.macOS.identity && !process.env['VOLT_MACOS_SIGNING_IDENTITY']) {
        console.warn(
          `[volt] Warning in ${filename}: 'package.signing.macOS.identity' is not set and VOLT_MACOS_SIGNING_IDENTITY env var is missing. macOS signing will be skipped.`,
        );
      }
      if (signing.macOS.entitlements && !existsSync(resolve(process.cwd(), signing.macOS.entitlements))) {
        const message = `'package.signing.macOS.entitlements' path does not exist: ${signing.macOS.entitlements}`;
        console.error(`[volt] Error in ${filename}: ${message}`);
        errors.push(message);
      }
    }

    if (signing.windows) {
      const providerInput = String(signing.windows.provider ?? process.env['VOLT_WIN_SIGNING_PROVIDER'] ?? 'local');
      let provider = normalizeWindowsSigningProvider(providerInput);

      if (provider === null) {
        const message =
          `'package.signing.windows.provider' must be one of: local, azureTrustedSigning, digicertKeyLocker.`;
        console.error(`[volt] Error in ${filename}: ${message}`);
        errors.push(message);
        provider = 'local';
      }

      if (!signing.windows.certificate && !process.env['VOLT_WIN_CERTIFICATE']) {
        if (provider === 'local') {
          console.warn(
            `[volt] Warning in ${filename}: 'package.signing.windows.certificate' is not set and VOLT_WIN_CERTIFICATE env var is missing. Windows signing will be skipped.`,
          );
        }
      }
      if (signing.windows.digestAlgorithm && !VALID_DIGEST_ALGORITHMS.includes(signing.windows.digestAlgorithm)) {
        const message =
          `'package.signing.windows.digestAlgorithm' must be one of: ${VALID_DIGEST_ALGORITHMS.join(', ')}. Got: '${signing.windows.digestAlgorithm}'.`;
        console.error(`[volt] Error in ${filename}: ${message}`);
        errors.push(message);
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
          console.warn(
            `[volt] Warning in ${filename}: Azure Trusted Signing selected but dlib/metadata path is missing. Set package.signing.windows.azureTrustedSigning.{dlibPath,metadataPath} or VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH and VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH.`,
          );
        }
      }

      if (provider === 'digicertKeyLocker') {
        const keypairAlias =
          signing.windows.digicertKeyLocker?.keypairAlias
          ?? process.env['VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS'];
        if (!keypairAlias) {
          console.warn(
            `[volt] Warning in ${filename}: DigiCert KeyLocker selected but keypair alias is missing. Set package.signing.windows.digicertKeyLocker.keypairAlias or VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS.`,
          );
        }
      }
    }
  }

  if (options.strict && errors.length > 0) {
    throw new Error(
      `[volt] Invalid configuration in ${filename}:\n${errors.map((error) => `- ${error}`).join('\n')}`,
    );
  }

  return config;
}

function normalizeWindowsInstallMode(value: unknown): 'perMachine' | 'perUser' | null {
  if (typeof value !== 'string') {
    return null;
  }

  const normalized = value.trim().toLowerCase();
  if (normalized === 'permachine' || normalized === 'per-machine') {
    return 'perMachine';
  }
  if (normalized === 'peruser' || normalized === 'per-user') {
    return 'perUser';
  }
  return null;
}

function isValidUpdaterEndpoint(value: string): boolean {
  try {
    const parsed = new URL(value.trim());
    if (parsed.protocol === 'https:') {
      return true;
    }
    if (parsed.protocol !== 'http:') {
      return false;
    }

    const hostname = parsed.hostname.toLowerCase().replace(/^\[|\]$/g, '');
    return hostname === 'localhost' || hostname === '::1' || isLoopbackIpv4(hostname);
  } catch {
    return false;
  }
}

function isLoopbackIpv4(hostname: string): boolean {
  const octets = hostname.split('.');
  if (octets.length !== LOOPBACK_IPV4_OCTET_COUNT) {
    return false;
  }

  const numbers = octets.map((segment) => Number.parseInt(segment, 10));
  if (numbers.some((value) => Number.isNaN(value) || value < 0 || value > 255)) {
    return false;
  }

  return numbers[0] === 127;
}

function isValidEd25519PublicKey(value: string): boolean {
  const trimmed = value.trim();
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(trimmed) || trimmed.length % 4 !== 0) {
    return false;
  }

  try {
    const decoded = Buffer.from(trimmed, 'base64');
    return decoded.length === BASE64_ED25519_PUBLIC_KEY_LENGTH
      && decoded.toString('base64') === trimmed;
  } catch {
    return false;
  }
}
