import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolveSigningConfig, signWindows } from '../utils/signing.js';

const originalPlatform = process.platform;
const originalEnv = { ...process.env };

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(existsSync).mockReturnValue(true);
  vi.mocked(execFileSync).mockReturnValue(Buffer.from(''));

  for (const key of Object.keys(process.env)) {
    if (key.startsWith('VOLT_') || key.startsWith('APPLE_')) {
      delete process.env[key];
    }
  }
});

afterEach(() => {
  process.env = { ...originalEnv };
  Object.defineProperty(process, 'platform', { value: originalPlatform });
});

describe('signing provider integration harness (mocked)', () => {
  it('runs Azure Trusted Signing flow with signtool dlib + metadata', async () => {
    Object.defineProperty(process, 'platform', { value: 'win32' });
    process.env['VOLT_AZURE_TRUSTED_SIGNING_DLIB_PATH'] = 'C:\\azure\\Azure.CodeSigning.Dlib.dll';
    process.env['VOLT_AZURE_TRUSTED_SIGNING_METADATA_PATH'] = 'C:\\azure\\metadata.json';

    const resolved = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'azureTrustedSigning',
          },
        },
      },
      'win32',
    );
    expect(resolved?.windows?.provider).toBe('azureTrustedSigning');

    const result = await signWindows('C:\\app\\my-app.exe', resolved!.windows!);
    expect(result.provider).toBe('azureTrustedSigning');
    expect(result.tool).toBe('signtool');
    expect(result.signed).toBe(true);

    const signCall = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => command === 'signtool' && Array.isArray(args) && args[0] === 'sign',
    );
    expect(signCall).toBeDefined();
    const signArgs = signCall?.[1] as string[];
    expect(signArgs).toContain('/dlib');
    expect(signArgs).toContain('C:\\azure\\Azure.CodeSigning.Dlib.dll');
    expect(signArgs).toContain('/dmdf');
    expect(signArgs).toContain('C:\\azure\\metadata.json');
  });

  it('fails Azure Trusted Signing when dlib/metadata is missing', async () => {
    Object.defineProperty(process, 'platform', { value: 'win32' });

    const resolved = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'azureTrustedSigning',
          },
        },
      },
      'win32',
    );
    await expect(signWindows('C:\\app\\my-app.exe', resolved!.windows!)).rejects.toThrow(
      'Azure Trusted Signing requires both dlibPath and metadataPath',
    );
  });

  it('runs DigiCert KeyLocker flow with smctl', async () => {
    process.env['VOLT_WIN_SIGNING_PROVIDER'] = 'digicertKeyLocker';
    process.env['VOLT_DIGICERT_KEYLOCKER_KEYPAIR_ALIAS'] = 'my-keypair';
    process.env['VOLT_DIGICERT_KEYLOCKER_CERT_FINGERPRINT'] = 'ABCD1234';
    process.env['VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH'] = 'smctl';

    const resolved = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'digicertKeyLocker',
          },
        },
      },
      'win32',
    );
    expect(resolved?.windows?.provider).toBe('digicertKeyLocker');

    const result = await signWindows('/path/to/app.exe', resolved!.windows!);
    expect(result.provider).toBe('digicertKeyLocker');
    expect(result.tool).toBe('smctl');

    const signCall = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => command === 'smctl' && Array.isArray(args) && args[0] === 'sign',
    );
    expect(signCall).toBeDefined();
    const signArgs = signCall?.[1] as string[];
    expect(signArgs).toContain('--keypair-alias');
    expect(signArgs).toContain('my-keypair');
    expect(signArgs).toContain('--certificate-fingerprint');
    expect(signArgs).toContain('ABCD1234');

    const verifyCall = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => (
        Array.isArray(args)
        && args[0] === 'verify'
        && (command === 'osslsigncode' || command === 'signtool')
      ),
    );
    expect(verifyCall).toBeDefined();
  });

  it('fails DigiCert KeyLocker flow when keypair alias is missing', async () => {
    process.env['VOLT_WIN_SIGNING_PROVIDER'] = 'digicertKeyLocker';
    process.env['VOLT_DIGICERT_KEYLOCKER_SMCTL_PATH'] = 'smctl';

    const resolved = resolveSigningConfig(
      {
        signing: {
          windows: {
            provider: 'digicertKeyLocker',
          },
        },
      },
      'win32',
    );
    await expect(signWindows('/path/to/app.exe', resolved!.windows!)).rejects.toThrow(
      'DigiCert KeyLocker signing requires keypairAlias',
    );
  });
});
