import { pushError, type ValidationContext } from './context.js';
import { isPlainObject } from './shared.js';

export function validatePackageConfig(
  configRecord: Record<string, unknown>,
  context: ValidationContext,
): void {
  const packageRecord = configRecord.package as Record<string, unknown> | undefined;
  if (!packageRecord) {
    return;
  }

  const windowsRecord = packageRecord.windows as Record<string, unknown> | undefined;
  if (windowsRecord !== undefined && !isPlainObject(windowsRecord)) {
    pushError(context, `'package.windows' must be an object when provided.`);
    delete packageRecord.windows;
  } else if (windowsRecord) {
    const installMode = windowsRecord.installMode;
    if (installMode !== undefined) {
      const normalizedInstallMode = normalizeWindowsInstallMode(installMode);
      if (!normalizedInstallMode) {
        pushError(context, `'package.windows.installMode' must be "perMachine" or "perUser".`);
        delete windowsRecord.installMode;
      } else {
        windowsRecord.installMode = normalizedInstallMode;
      }
    }

    const silentAllUsers = windowsRecord.silentAllUsers;
    if (silentAllUsers !== undefined && typeof silentAllUsers !== 'boolean') {
      pushError(context, `'package.windows.silentAllUsers' must be a boolean when provided.`);
      delete windowsRecord.silentAllUsers;
    }

    const msixRecord = windowsRecord.msix as Record<string, unknown> | undefined;
    if (msixRecord !== undefined && !isPlainObject(msixRecord)) {
      pushError(context, `'package.windows.msix' must be an object when provided.`);
      delete windowsRecord.msix;
    } else if (msixRecord) {
      for (const field of ['identityName', 'publisher', 'publisherDisplayName', 'displayName', 'description']) {
        const value = msixRecord[field];
        if (value !== undefined && typeof value !== 'string') {
          pushError(context, `'package.windows.msix.${field}' must be a string when provided.`);
          delete msixRecord[field];
        }
      }
    }
  }

  const enterpriseRecord = packageRecord.enterprise as Record<string, unknown> | undefined;
  if (enterpriseRecord !== undefined && !isPlainObject(enterpriseRecord)) {
    pushError(context, `'package.enterprise' must be an object when provided.`);
    delete packageRecord.enterprise;
  } else if (enterpriseRecord) {
    for (const field of ['generateAdmx', 'includeDocsBundle']) {
      const value = enterpriseRecord[field];
      if (value !== undefined && typeof value !== 'boolean') {
        pushError(context, `'package.enterprise.${field}' must be a boolean when provided.`);
        delete enterpriseRecord[field];
      }
    }
  }
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
