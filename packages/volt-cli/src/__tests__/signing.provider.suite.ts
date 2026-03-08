import { describe, expect, it } from 'vitest';
import {
  normalizeWindowsSigningProvider,
  resolveWindowsSigningProvider,
} from '../utils/signing/provider.js';

describe('windows signing provider normalization', () => {
  it('normalizes supported aliases', () => {
    expect(normalizeWindowsSigningProvider('local')).toBe('local');
    expect(normalizeWindowsSigningProvider('azure')).toBe('azureTrustedSigning');
    expect(normalizeWindowsSigningProvider('azure_trusted_signing')).toBe('azureTrustedSigning');
    expect(normalizeWindowsSigningProvider('digicert')).toBe('digicertKeyLocker');
    expect(normalizeWindowsSigningProvider('keylocker')).toBe('digicertKeyLocker');
    expect(normalizeWindowsSigningProvider(' DIGICERT_KEYLOCKER ')).toBe('digicertKeyLocker');
  });

  it('returns null for unsupported values', () => {
    expect(normalizeWindowsSigningProvider(undefined)).toBeNull();
    expect(normalizeWindowsSigningProvider('')).toBeNull();
    expect(normalizeWindowsSigningProvider('unknown-provider')).toBeNull();
  });

  it('falls back when provider is unsupported or empty', () => {
    expect(resolveWindowsSigningProvider(undefined)).toBe('local');
    expect(resolveWindowsSigningProvider('')).toBe('local');
    expect(resolveWindowsSigningProvider('unknown')).toBe('local');
    expect(resolveWindowsSigningProvider('unknown', 'azureTrustedSigning')).toBe('azureTrustedSigning');
  });
});
