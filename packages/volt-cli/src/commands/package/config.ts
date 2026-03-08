import type { MacOSSigningConfig, WindowsSigningConfig } from 'voltkit';
import {
  ALLOWED_PACKAGE_FORMATS,
  type PackageConfig,
  type PackageEnterpriseConfig,
  type PackageWindowsConfig,
  type PackageWindowsMsixConfig,
  type WindowsInstallMode,
} from './types.js';

export function parsePackageConfig(raw: unknown, appName: string): PackageConfig {
  const fallback: PackageConfig = {
    identifier: `com.volt.${appName.toLowerCase().replace(/\s+/g, '-')}`,
  };

  if (!raw || typeof raw !== 'object') {
    return fallback;
  }

  const value = raw as Record<string, unknown>;
  const identifier = typeof value.identifier === 'string' && value.identifier.trim()
    ? value.identifier.trim()
    : fallback.identifier;
  const icon = typeof value.icon === 'string' && value.icon.trim() ? value.icon : undefined;
  const categories = Array.isArray(value.categories)
    ? value.categories.filter((entry): entry is string => typeof entry === 'string' && entry.trim().length > 0)
    : undefined;
  const windows = parseWindowsConfig(value.windows);
  const enterprise = parseEnterpriseConfig(value.enterprise);

  let signing: PackageConfig['signing'];
  if (value.signing && typeof value.signing === 'object') {
    const signingValue = value.signing as Record<string, unknown>;
    signing = {};
    if (signingValue.macOS && typeof signingValue.macOS === 'object') {
      signing.macOS = signingValue.macOS as MacOSSigningConfig;
    }
    if (signingValue.windows && typeof signingValue.windows === 'object') {
      signing.windows = signingValue.windows as WindowsSigningConfig;
    }
    if (!signing.macOS && !signing.windows) {
      signing = undefined;
    }
  }

  return {
    identifier,
    icon,
    categories,
    windows,
    enterprise,
    signing,
  };
}

export function validateRequestedPackageFormat(
  platform: 'win32' | 'darwin' | 'linux',
  format: string | undefined,
): string | undefined {
  if (!format) {
    return undefined;
  }
  const normalized = format.trim().toLowerCase();
  const supported = ALLOWED_PACKAGE_FORMATS[platform];
  if (!supported.includes(normalized)) {
    return undefined;
  }
  return normalized;
}

function parseWindowsConfig(raw: unknown): PackageWindowsConfig | undefined {
  if (!raw || typeof raw !== 'object') {
    return undefined;
  }

  const value = raw as Record<string, unknown>;
  const installMode = normalizeWindowsInstallMode(value.installMode);
  const silentAllUsers = typeof value.silentAllUsers === 'boolean' ? value.silentAllUsers : undefined;
  const msix = parseWindowsMsixConfig(value.msix);

  if (!installMode && silentAllUsers === undefined && !msix) {
    return undefined;
  }

  return {
    installMode,
    silentAllUsers,
    msix,
  };
}

function parseWindowsMsixConfig(raw: unknown): PackageWindowsMsixConfig | undefined {
  if (!raw || typeof raw !== 'object') {
    return undefined;
  }

  const value = raw as Record<string, unknown>;
  const identityName = normalizeNonEmptyString(value.identityName);
  const publisher = normalizeNonEmptyString(value.publisher);
  const publisherDisplayName = normalizeNonEmptyString(value.publisherDisplayName);
  const displayName = normalizeNonEmptyString(value.displayName);
  const description = normalizeNonEmptyString(value.description);

  if (!identityName && !publisher && !publisherDisplayName && !displayName && !description) {
    return undefined;
  }

  return {
    identityName,
    publisher,
    publisherDisplayName,
    displayName,
    description,
  };
}

function parseEnterpriseConfig(raw: unknown): PackageEnterpriseConfig | undefined {
  if (!raw || typeof raw !== 'object') {
    return undefined;
  }

  const value = raw as Record<string, unknown>;
  const generateAdmx = typeof value.generateAdmx === 'boolean' ? value.generateAdmx : undefined;
  const includeDocsBundle = typeof value.includeDocsBundle === 'boolean' ? value.includeDocsBundle : undefined;

  if (generateAdmx === undefined && includeDocsBundle === undefined) {
    return undefined;
  }

  return {
    generateAdmx,
    includeDocsBundle,
  };
}

export function normalizeWindowsInstallMode(value: unknown): WindowsInstallMode | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }

  const normalized = value.trim().toLowerCase();
  if (normalized === 'permachine' || normalized === 'per-machine') {
    return 'perMachine';
  }
  if (normalized === 'peruser' || normalized === 'per-user') {
    return 'perUser';
  }

  return undefined;
}

function normalizeNonEmptyString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

export const __testOnly = {
  normalizeWindowsInstallMode,
};
