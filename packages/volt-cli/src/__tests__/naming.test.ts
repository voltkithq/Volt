import { describe, it, expect } from 'vitest';
import { toSafeArtifactVersion, toSafeBinaryName } from '../utils/naming.js';

describe('naming safety helpers', () => {
  it('rejects app names without alphanumeric content', () => {
    expect(() => toSafeBinaryName('////')).toThrow(
      'Application name must contain at least one alphanumeric character.',
    );
  });

  it('removes path separators and shell metacharacters from binary names', () => {
    expect(toSafeBinaryName('../My App && calc.exe')).toBe('my-app-calc.exe');
  });

  it('normalizes artifact versions to filename-safe values', () => {
    expect(toSafeArtifactVersion('1.2.3+build/5')).toBe('1.2.3+build-5');
  });
});
