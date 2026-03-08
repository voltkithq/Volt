import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/test.js';

describe('test command helpers', () => {
  it('normalizes suite names from strings and arrays', () => {
    expect(__testOnly.normalizeSuiteNames(undefined)).toBeUndefined();
    expect(__testOnly.normalizeSuiteNames(' ipc-demo ')).toEqual(['ipc-demo']);
    expect(__testOnly.normalizeSuiteNames(['a', '  ', 'b'])).toEqual(['a', 'b']);
  });

  it('parses timeout values', () => {
    expect(__testOnly.parseTimeoutMs(undefined)).toBeUndefined();
    expect(__testOnly.parseTimeoutMs('1234')).toBe(1234);
    expect(() => __testOnly.parseTimeoutMs('0')).toThrow('Invalid --timeout');
    expect(() => __testOnly.parseTimeoutMs('abc')).toThrow('Invalid --timeout');
  });

  it('parses retry values', () => {
    expect(__testOnly.parseRetryCount(undefined)).toBeUndefined();
    expect(__testOnly.parseRetryCount('0')).toBe(0);
    expect(__testOnly.parseRetryCount('2')).toBe(2);
    expect(() => __testOnly.parseRetryCount('-1')).toThrow('Invalid --retries');
    expect(() => __testOnly.parseRetryCount('1.5')).toThrow('Invalid --retries');
  });

  it('resolves the CLI entry path inside dist', () => {
    const entryPath = __testOnly.resolveCliEntryPath();
    expect(entryPath).toMatch(/index\.js$/);
  });
});
