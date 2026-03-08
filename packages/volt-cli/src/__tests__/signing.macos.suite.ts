import { beforeEach, describe, expect, it, vi } from 'vitest';
import { signMacOS, type ResolvedMacOSConfig } from '../utils/signing.js';
import { execFileSync } from 'node:child_process';

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(execFileSync).mockReturnValue(Buffer.from(''));
});

describe('signMacOS', () => {
  it('throws if codesign is not available', async () => {
    vi.mocked(execFileSync).mockImplementation((command, args) => {
      if (
        typeof command === 'string'
        && (command === 'which' || command === 'where')
        && Array.isArray(args)
        && args.includes('codesign')
      ) {
        throw new Error('not found');
      }
      return Buffer.from('');
    });

    const config: ResolvedMacOSConfig = {
      identity: 'Test Identity',
      notarize: false,
    };
    await expect(signMacOS('/path/to/App.app', config)).rejects.toThrow('codesign not found');
  });

  it('calls codesign with correct arguments', async () => {
    const config: ResolvedMacOSConfig = {
      identity: 'Developer ID Application: Test (ABC)',
      notarize: false,
    };
    await signMacOS('/path/to/App.app', config);

    const call = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => command === 'codesign' && Array.isArray(args) && args.includes('--sign'),
    );
    expect(call).toBeDefined();
    const args = call?.[1] as string[];
    expect(args).toContain('--deep');
    expect(args).toContain('--force');
    expect(args).toContain('--options');
    expect(args).toContain('runtime');
    expect(args).toContain('--timestamp');
    expect(args).toContain('Developer ID Application: Test (ABC)');
    expect(args).toContain('/path/to/App.app');
  });

  it('includes entitlements when configured', async () => {
    const config: ResolvedMacOSConfig = {
      identity: 'Test',
      entitlements: './my-entitlements.plist',
      notarize: false,
    };
    await signMacOS('/path/to/App.app', config);

    const call = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => command === 'codesign' && Array.isArray(args) && args.includes('--entitlements'),
    );
    expect(call).toBeDefined();
    const args = call?.[1] as string[];
    expect(args).toContain('./my-entitlements.plist');
  });

  it('verifies signature after signing', async () => {
    const config: ResolvedMacOSConfig = {
      identity: 'Test',
      notarize: false,
    };
    await signMacOS('/path/to/App.app', config);

    const call = vi.mocked(execFileSync).mock.calls.find(
      ([command, args]) => command === 'codesign' && Array.isArray(args) && args.includes('--verify'),
    );
    expect(call).toBeDefined();
    const args = call?.[1] as string[];
    expect(args).toContain('--deep');
    expect(args).toContain('--strict');
  });

  it('skips notarization when notarize is false', async () => {
    const config: ResolvedMacOSConfig = {
      identity: 'Test',
      notarize: false,
    };
    await signMacOS('/path/to/App.app', config);

    const calls = vi.mocked(execFileSync).mock.calls;
    const notaryCall = calls.find(
      ([command, args]) => command === 'xcrun' && Array.isArray(args) && args[0] === 'notarytool',
    );
    expect(notaryCall).toBeUndefined();
  });

  it('warns and skips notarization when credentials are missing', async () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const config: ResolvedMacOSConfig = {
      identity: 'Test',
      notarize: true,
      // Missing appleId, applePassword, teamId
    };
    await signMacOS('/path/to/App.app', config);

    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining('Skipping notarization'),
    );

    const calls = vi.mocked(execFileSync).mock.calls;
    const notaryCall = calls.find(
      ([command, args]) => command === 'xcrun' && Array.isArray(args) && args[0] === 'notarytool',
    );
    expect(notaryCall).toBeUndefined();
    consoleSpy.mockRestore();
  });

  it('calls notarytool when notarize is true and credentials are present', async () => {
    const config: ResolvedMacOSConfig = {
      identity: 'Test',
      notarize: true,
      appleId: 'user@example.com',
      applePassword: 'app-specific-pw',
      teamId: 'TEAM123',
    };
    await signMacOS('/path/to/App.app', config);

    const calls = vi.mocked(execFileSync).mock.calls;
    const dittoCall = calls.find(([command]) => command === 'ditto');
    const notaryCall = calls.find(
      ([command, args]) => command === 'xcrun' && Array.isArray(args) && args[0] === 'notarytool',
    );
    const staplerCall = calls.find(
      ([command, args]) => command === 'xcrun' && Array.isArray(args) && args[0] === 'stapler',
    );

    expect(dittoCall).toBeDefined();
    expect(notaryCall).toBeDefined();
    const notaryArgs = notaryCall?.[1] as string[];
    expect(notaryArgs).toContain('user@example.com');
    expect(notaryArgs).toContain('TEAM123');
    expect(notaryArgs).toContain('--wait');
    expect(staplerCall).toBeDefined();
  });
});
