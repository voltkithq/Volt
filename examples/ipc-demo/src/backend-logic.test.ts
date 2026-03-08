import { describe, expect, it } from 'vitest';
import {
  evaluateNativeReady,
  extractDbRowsCount,
  formatUuidFromHash,
  normalizeSecretKey,
  normalizeSecretValue,
  summarizeClipboardRead,
  toDbRecords,
  toFiniteNumber,
} from './backend-logic.js';

describe('ipc-demo backend logic', () => {
  it('validates finite compute arguments', () => {
    expect(toFiniteNumber(42, 'a')).toBe(42);
    expect(() => toFiniteNumber(Number.NaN, 'a')).toThrow('compute.a must be a finite number');
    expect(() => toFiniteNumber('42', 'b')).toThrow('compute.b must be a finite number');
  });

  it('formats stable UUID-like strings from hash input', () => {
    const uuid = formatUuidFromHash('1234567890abcdef1234567890abcdefabcdef');
    expect(uuid).toBe('12345678-90ab-cdef-1234-567890abcdef');
  });

  it('maps database rows and filters invalid rows', () => {
    const records = toDbRecords([
      { id: '1', message: 'hello', created_at: 100 },
      { id: '2', message: 'world', created_at: '200' },
      { id: 3, message: 'bad' },
    ]);
    expect(records).toEqual([
      { id: '1', message: 'hello', createdAt: 100 },
      { id: '2', message: 'world', createdAt: 200 },
    ]);
  });

  it('extracts row count safely from aggregate row payload', () => {
    expect(extractDbRowsCount({ total: 7 })).toBe(7);
    expect(extractDbRowsCount({ total: '9' })).toBe(9);
    expect(extractDbRowsCount({ total: 'oops' })).toBe(0);
    expect(extractDbRowsCount(null)).toBe(0);
  });

  it('summarizes clipboard reads without mutating clipboard state', () => {
    expect(summarizeClipboardRead('copied text')).toEqual({
      read: 'copied text',
      hasText: true,
    });
    expect(summarizeClipboardRead('')).toEqual({
      read: '',
      hasText: false,
    });
    expect(summarizeClipboardRead(null)).toEqual({
      read: '',
      hasText: false,
    });
  });

  it('evaluates native integration readiness from feature state', () => {
    expect(
      evaluateNativeReady({
        menuConfigured: true,
        shortcutRegistered: true,
        trayReady: true,
      }),
    ).toBe(true);
    expect(
      evaluateNativeReady({
        menuConfigured: true,
        shortcutRegistered: true,
        trayReady: false,
      }),
    ).toBe(false);
  });

  it('normalizes secure storage key payloads', () => {
    expect(normalizeSecretKey(' demo/token ')).toBe('demo/token');
    expect(() => normalizeSecretKey('   ')).toThrow('secureStorage.key must not be empty');
    expect(() => normalizeSecretKey(42)).toThrow('secureStorage.key must be a string');
  });

  it('validates secure storage value payloads', () => {
    expect(normalizeSecretValue('s3cr3t')).toBe('s3cr3t');
    expect(() => normalizeSecretValue('    ')).toThrow('secureStorage.value must not be empty');
    expect(() => normalizeSecretValue(false)).toThrow('secureStorage.value must be a string');
  });
});
