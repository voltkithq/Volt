import { describe, expect, it } from 'vitest';

import { validatePluginManifest } from '../../utils/plugin-manifest.js';

import { validManifest } from './fixtures.js';

describe('validatePluginManifest capabilities and signature fields', () => {
  it('rejects missing capabilities', () => {
    const { capabilities: _capabilities, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'capabilities')).toBe(true);
  });

  it('rejects capabilities as string', () => {
    const result = validatePluginManifest(validManifest({ capabilities: 'fs' }));
    expect(result.valid).toBe(false);
  });

  it('rejects unknown capability', () => {
    const result = validatePluginManifest(validManifest({ capabilities: ['fs', 'teleport'] }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.message.includes('Unknown capability'))).toBe(true);
  });

  it('rejects non-string element in capabilities', () => {
    const result = validatePluginManifest(validManifest({ capabilities: [42] }));
    expect(result.valid).toBe(false);
  });

  it('rejects duplicate capabilities', () => {
    const result = validatePluginManifest(validManifest({ capabilities: ['fs', 'fs'] }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.message.includes('Duplicate'))).toBe(true);
  });

  it('accepts a valid signature', () => {
    const result = validatePluginManifest(
      validManifest({
        signature: { algorithm: 'ed25519', value: 'abc123def456' },
      }),
    );
    expect(result.valid).toBe(true);
    expect(result.manifest!.signature).toEqual({
      algorithm: 'ed25519',
      value: 'abc123def456',
    });
  });

  it('accepts manifest without signature', () => {
    const result = validatePluginManifest(validManifest());
    expect(result.valid).toBe(true);
    expect(result.manifest!.signature).toBeUndefined();
  });

  it('rejects signature as non-object', () => {
    const result = validatePluginManifest(validManifest({ signature: 'bad' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'signature')).toBe(true);
  });

  it('rejects signature with missing algorithm', () => {
    const result = validatePluginManifest(validManifest({ signature: { value: 'abc123' } }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'signature.algorithm')).toBe(true);
  });

  it('rejects signature with missing value', () => {
    const result = validatePluginManifest(validManifest({ signature: { algorithm: 'ed25519' } }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'signature.value')).toBe(true);
  });

  it('rejects signature with empty algorithm', () => {
    const result = validatePluginManifest(
      validManifest({ signature: { algorithm: '', value: 'abc' } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'signature.algorithm')).toBe(true);
  });
});
