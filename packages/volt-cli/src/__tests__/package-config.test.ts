import { describe, expect, it } from 'vitest';
import {
  normalizeWindowsInstallMode,
  parsePackageConfig,
  validateRequestedPackageFormat,
} from '../commands/package/config.js';

describe('package config parsing', () => {
  it('parses Windows install mode and MSIX metadata', () => {
    const parsed = parsePackageConfig(
      {
        identifier: 'com.example.app',
        windows: {
          installMode: 'per-machine',
          silentAllUsers: true,
          msix: {
            identityName: 'com.example.app',
            publisher: 'CN=Example Corp',
            displayName: 'Example App',
          },
        },
      },
      'Example App',
    );

    expect(parsed.identifier).toBe('com.example.app');
    expect(parsed.windows?.installMode).toBe('perMachine');
    expect(parsed.windows?.silentAllUsers).toBe(true);
    expect(parsed.windows?.msix?.identityName).toBe('com.example.app');
    expect(parsed.windows?.msix?.publisher).toBe('CN=Example Corp');
  });

  it('parses enterprise options and defaults identifier when missing', () => {
    const parsed = parsePackageConfig(
      {
        enterprise: {
          generateAdmx: false,
          includeDocsBundle: true,
        },
      },
      'My Great App',
    );

    expect(parsed.identifier).toBe('com.volt.my-great-app');
    expect(parsed.enterprise).toEqual({
      generateAdmx: false,
      includeDocsBundle: true,
    });
  });

  it('normalizes install mode aliases and validates platform format support', () => {
    expect(normalizeWindowsInstallMode('perUser')).toBe('perUser');
    expect(normalizeWindowsInstallMode('per-user')).toBe('perUser');
    expect(normalizeWindowsInstallMode('perMachine')).toBe('perMachine');
    expect(normalizeWindowsInstallMode('per-machine')).toBe('perMachine');
    expect(normalizeWindowsInstallMode('invalid')).toBeUndefined();

    expect(validateRequestedPackageFormat('win32', 'msix')).toBe('msix');
    expect(validateRequestedPackageFormat('win32', 'appimage')).toBeUndefined();
  });
});
