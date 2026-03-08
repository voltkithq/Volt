import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isToolAvailable } from '../utils/signing.js';
import { execFileSync } from 'node:child_process';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('isToolAvailable', () => {
  it('returns true when tool is found', () => {
    vi.mocked(execFileSync).mockReturnValueOnce(Buffer.from('/usr/bin/codesign'));
    expect(isToolAvailable('codesign')).toBe(true);
  });

  it('returns false when tool is not found', () => {
    vi.mocked(execFileSync).mockImplementationOnce(() => {
      throw new Error('not found');
    });
    expect(isToolAvailable('nonexistent-tool')).toBe(false);
  });
});
