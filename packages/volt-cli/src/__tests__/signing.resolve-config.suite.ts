import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { resolveSigningConfig } from '../utils/signing.js';

const originalEnv = { ...process.env };

beforeEach(() => {
  for (const key of Object.keys(process.env)) {
    if (key.startsWith('VOLT_') || key.startsWith('APPLE_')) {
      delete process.env[key];
    }
  }
});

afterEach(() => {
  process.env = { ...originalEnv };
});

describe('resolveSigningConfig', () => {
  it('returns null when no signing config and no env vars', () => {
    const result = resolveSigningConfig(undefined, 'darwin');
    expect(result).toBeNull();
  });

  it('returns null for empty package config', () => {
    const result = resolveSigningConfig({}, 'darwin');
    expect(result).toBeNull();
  });

  it('returns null for Linux (no signing needed)', () => {
    const result = resolveSigningConfig(
      { signing: { macOS: { identity: 'test' } } },
      'linux',
    );
    expect(result).toBeNull();
  });

  it('reads macOS config from volt.config', () => {
    const result = resolveSigningConfig(
      {
        signing: {
          macOS: {
            identity: 'Developer ID Application: Test (ABC123)',
            entitlements: './entitlements.plist',
            notarize: false,
            teamId: 'ABC123',
          },
        },
      },
      'darwin',
    );
    expect(result).not.toBeNull();
    expect(result!.macOS).toBeDefined();
    expect(result!.macOS!.identity).toBe('Developer ID Application: Test (ABC123)');
    expect(result!.macOS!.entitlements).toBe('./entitlements.plist');
    expect(result!.macOS!.notarize).toBe(false);
    expect(result!.macOS!.teamId).toBe('ABC123');
  });

  it('macOS env vars override config values', () => {
    process.env['VOLT_MACOS_SIGNING_IDENTITY'] = 'Env Identity';
    process.env['VOLT_APPLE_TEAM_ID'] = 'ENV_TEAM';
    process.env['VOLT_APPLE_ID'] = 'user@example.com';
    process.env['VOLT_APPLE_PASSWORD'] = 'secret';
    process.env['VOLT_MACOS_CERTIFICATE'] = 'base64cert';
    process.env['VOLT_MACOS_CERTIFICATE_PASSWORD'] = 'certpass';

    const result = resolveSigningConfig(
      { signing: { macOS: { identity: 'Config Identity', teamId: 'CONFIG_TEAM' } } },
      'darwin',
    );
    expect(result!.macOS!.identity).toBe('Env Identity');
    expect(result!.macOS!.teamId).toBe('ENV_TEAM');
    expect(result!.macOS!.appleId).toBe('user@example.com');
    expect(result!.macOS!.applePassword).toBe('secret');
    expect(result!.macOS!.certificate).toBe('base64cert');
    expect(result!.macOS!.certificatePassword).toBe('certpass');
  });

  it('macOS identity from env var alone is sufficient', () => {
    process.env['VOLT_MACOS_SIGNING_IDENTITY'] = 'Env Only Identity';
    const result = resolveSigningConfig(undefined, 'darwin');
    expect(result).not.toBeNull();
    expect(result!.macOS!.identity).toBe('Env Only Identity');
  });

  it('macOS notarize defaults to true', () => {
    const result = resolveSigningConfig(
      { signing: { macOS: { identity: 'Test' } } },
      'darwin',
    );
    expect(result!.macOS!.notarize).toBe(true);
  });

  it('resolves for platform string containing "apple"', () => {
    const result = resolveSigningConfig(
      { signing: { macOS: { identity: 'Test' } } },
      'apple-darwin',
    );
    expect(result).not.toBeNull();
    expect(result!.macOS).toBeDefined();
  });

  it('reads Windows config from volt.config', () => {
    const result = resolveSigningConfig(
      {
        signing: {
          windows: {
            certificate: './cert.pfx',
            digestAlgorithm: 'sha384',
            timestampUrl: 'http://timestamp.sectigo.com',
          },
        },
      },
      'win32',
    );
    expect(result).not.toBeNull();
    expect(result!.windows).toBeDefined();
    expect(result!.windows!.provider).toBe('local');
    expect(result!.windows!.certificate).toBe('./cert.pfx');
    expect(result!.windows!.digestAlgorithm).toBe('sha384');
    expect(result!.windows!.timestampUrl).toBe('http://timestamp.sectigo.com');
  });

  it('Windows env vars override config values', () => {
    process.env['VOLT_WIN_CERTIFICATE'] = '/env/cert.pfx';
    process.env['VOLT_WIN_CERTIFICATE_PASSWORD'] = 'envpass';

    const result = resolveSigningConfig(
      { signing: { windows: { certificate: './config/cert.pfx' } } },
      'win32',
    );
    expect(result!.windows!.certificate).toBe('/env/cert.pfx');
    expect(result!.windows!.certificatePassword).toBe('envpass');
  });

  it('Windows defaults digestAlgorithm to sha256', () => {
    const result = resolveSigningConfig(
      { signing: { windows: { certificate: './cert.pfx' } } },
      'win32',
    );
    expect(result!.windows!.digestAlgorithm).toBe('sha256');
  });

  it('Windows defaults timestampUrl to digicert', () => {
    const result = resolveSigningConfig(
      { signing: { windows: { certificate: './cert.pfx' } } },
      'win32',
    );
    expect(result!.windows!.timestampUrl).toBe('https://timestamp.digicert.com');
  });

  it('Windows certificate from env var alone is sufficient', () => {
    process.env['VOLT_WIN_CERTIFICATE'] = '/env/cert.pfx';
    const result = resolveSigningConfig(undefined, 'win32');
    expect(result).not.toBeNull();
    expect(result!.windows!.certificate).toBe('/env/cert.pfx');
  });

  it('resolves for platform string containing "windows"', () => {
    const result = resolveSigningConfig(
      { signing: { windows: { certificate: './cert.pfx' } } },
      'x86_64-windows-msvc',
    );
    expect(result).not.toBeNull();
    expect(result!.windows).toBeDefined();
  });

  it('returns null for Windows when no certificate configured', () => {
    const result = resolveSigningConfig(
      { signing: { windows: {} } },
      'win32',
    );
    expect(result).toBeNull();
  });

  it('normalizes provider aliases from env var', () => {
    process.env['VOLT_WIN_SIGNING_PROVIDER'] = 'azure_trusted_signing';
    const result = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'local',
            certificate: './cert.pfx',
          },
        },
      },
      'win32',
    );
    expect(result).not.toBeNull();
    expect(result!.windows!.provider).toBe('azureTrustedSigning');
  });

  it('falls back unknown provider to local for config resolution', () => {
    process.env['VOLT_WIN_SIGNING_PROVIDER'] = 'unsupported-provider';
    process.env['VOLT_WIN_CERTIFICATE'] = '/env/cert.pfx';
    const result = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'digicertKeyLocker',
          },
        },
      },
      'win32',
    );
    expect(result).not.toBeNull();
    expect(result!.windows!.provider).toBe('local');
    expect(result!.windows!.certificate).toBe('/env/cert.pfx');
  });

  it('resolves Azure Trusted Signing provider config', () => {
    const result = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'azureTrustedSigning',
            azureTrustedSigning: {
              dlibPath: './tools/Azure.CodeSigning.Dlib.dll',
              metadataPath: './tools/metadata.json',
              endpoint: 'https://eus.codesigning.azure.net',
            },
          },
        },
      },
      'win32',
    );

    expect(result).not.toBeNull();
    expect(result!.windows!.provider).toBe('azureTrustedSigning');
    expect(result!.windows!.azureTrustedSigning?.dlibPath).toBe('./tools/Azure.CodeSigning.Dlib.dll');
    expect(result!.windows!.azureTrustedSigning?.metadataPath).toBe('./tools/metadata.json');
    expect(result!.windows!.azureTrustedSigning?.endpoint).toBe('https://eus.codesigning.azure.net');
  });

  it('resolves DigiCert KeyLocker provider config', () => {
    const result = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'digicertKeyLocker',
            digicertKeyLocker: {
              keypairAlias: 'volt-release',
            },
          },
        },
      },
      'win32',
    );

    expect(result).not.toBeNull();
    expect(result!.windows!.provider).toBe('digicertKeyLocker');
    expect(result!.windows!.digicertKeyLocker?.keypairAlias).toBe('volt-release');
    expect(result!.windows!.digicertKeyLocker?.smctlPath).toBe('smctl');
  });
});
