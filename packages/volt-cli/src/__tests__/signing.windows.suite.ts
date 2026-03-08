import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { signWindows, type ResolvedWindowsConfig } from '../utils/signing.js';
import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';

const originalPlatform = process.platform;

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(existsSync).mockReturnValue(true);
  vi.mocked(execFileSync).mockReturnValue(Buffer.from(''));
});

afterEach(() => {
  Object.defineProperty(process, 'platform', { value: originalPlatform });
});

describe('signWindows', () => {
  it('throws if certificate file not found', async () => {
    vi.mocked(existsSync).mockReturnValue(false);
    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: '/missing/cert.pfx',
      digestAlgorithm: 'sha256',
      timestampUrl: 'http://timestamp.digicert.com',
    };
    await expect(signWindows('/path/to/app.exe', config)).rejects.toThrow(
      'Certificate file not found',
    );
  });

  it('throws if no signing tool is available', async () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(execFileSync).mockImplementation((command, args) => {
      if (
        typeof command === 'string'
        && (command === 'which' || command === 'where')
        && Array.isArray(args)
      ) {
        throw new Error('not found');
      }
      return Buffer.from('');
    });

    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: './cert.pfx',
      digestAlgorithm: 'sha256',
      timestampUrl: 'http://timestamp.digicert.com',
    };
    await expect(signWindows('/path/to/app.exe', config)).rejects.toThrow(
      'No signing tool found',
    );
  });

  it('uses signtool on win32 when available', async () => {
    Object.defineProperty(process, 'platform', { value: 'win32' });

    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: './cert.pfx',
      certificatePassword: 'mypass',
      digestAlgorithm: 'sha256',
      timestampUrl: 'http://timestamp.digicert.com',
    };
    await signWindows('/path/to/app.exe', config);

    const calls = vi.mocked(execFileSync).mock.calls;
    const signCall = calls.find(
      ([command, args]) => command === 'signtool' && Array.isArray(args) && args[0] === 'sign',
    );
    expect(signCall).toBeDefined();
    const signArgs = signCall?.[1] as string[];
    expect(signArgs).toContain('/f');
    expect(signArgs).toContain('./cert.pfx');
    expect(signArgs).toContain('/fd');
    expect(signArgs).toContain('sha256');
    expect(signArgs).toContain('/tr');
    expect(signArgs).toContain('http://timestamp.digicert.com');
    expect(signArgs).toContain('/p');

    const verifyCall = calls.find(
      ([command, args]) => command === 'signtool' && Array.isArray(args) && args[0] === 'verify',
    );
    expect(verifyCall).toBeDefined();
  });

  it('falls back to osslsigncode on non-win32', async () => {
    Object.defineProperty(process, 'platform', { value: 'linux' });

    vi.mocked(execFileSync).mockImplementation((command, args) => {
      if (
        typeof command === 'string'
        && command === 'which'
        && Array.isArray(args)
        && args.length > 0
      ) {
        const requested = String(args[0]);
        if (requested === 'signtool') {
          throw new Error('not found');
        }
        return Buffer.from('/usr/bin/osslsigncode');
      }
      return Buffer.from('');
    });

    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: './cert.pfx',
      digestAlgorithm: 'sha256',
      timestampUrl: 'http://timestamp.digicert.com',
    };
    await signWindows('/path/to/app.exe', config);

    const calls = vi.mocked(execFileSync).mock.calls;
    const signCall = calls.find(
      ([command, args]) => command === 'osslsigncode' && Array.isArray(args) && args[0] === 'sign',
    );
    expect(signCall).toBeDefined();
    const signArgs = signCall?.[1] as string[];
    expect(signArgs).toContain('-pkcs12');
    expect(signArgs).toContain('./cert.pfx');
    expect(signArgs).toContain('-h');
    expect(signArgs).toContain('sha256');

    const verifyCall = calls.find(
      ([command, args]) => command === 'osslsigncode' && Array.isArray(args) && args[0] === 'verify',
    );
    expect(verifyCall).toBeDefined();
  });

  it('uses correct timestamp and digest from config', async () => {
    Object.defineProperty(process, 'platform', { value: 'win32' });

    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: './cert.pfx',
      digestAlgorithm: 'sha384',
      timestampUrl: 'http://timestamp.sectigo.com',
    };
    await signWindows('/path/to/app.exe', config);

    const calls = vi.mocked(execFileSync).mock.calls;
    const signCall = calls.find(
      ([command, args]) => command === 'signtool' && Array.isArray(args) && args[0] === 'sign',
    );
    const signArgs = signCall?.[1] as string[];
    expect(signArgs).toContain('/fd');
    expect(signArgs).toContain('/td');
    expect(signArgs).toContain('sha384');
    expect(signArgs).toContain('/tr');
    expect(signArgs).toContain('http://timestamp.sectigo.com');
  });

  it('includes tool stdout and stderr when signing fails', async () => {
    Object.defineProperty(process, 'platform', { value: 'win32' });

    vi.mocked(execFileSync).mockImplementation((command, args) => {
      if (
        typeof command === 'string'
        && command === 'signtool'
        && Array.isArray(args)
        && args[0] === 'sign'
      ) {
        throw Object.assign(new Error('sign failed'), {
          status: 1,
          stdout: Buffer.from('mock stdout'),
          stderr: Buffer.from('mock stderr'),
        });
      }
      return Buffer.from('');
    });

    const config: ResolvedWindowsConfig = {
      provider: 'local',
      certificate: './cert.pfx',
      digestAlgorithm: 'sha256',
      timestampUrl: 'http://timestamp.digicert.com',
    };

    await expect(signWindows('/path/to/app.exe', config)).rejects.toThrow(
      /stdout: mock stdout[\s\S]*stderr: mock stderr/,
    );
  });
});
