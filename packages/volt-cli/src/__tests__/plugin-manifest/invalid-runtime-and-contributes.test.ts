import { describe, expect, it } from 'vitest';

import { validatePluginManifest } from '../../utils/plugin-manifest.js';

import { validManifest } from './fixtures.js';

describe('validatePluginManifest runtime and contributes fields', () => {
  it('rejects missing engine', () => {
    const { engine: _engine, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'engine')).toBe(true);
  });

  it('rejects engine without volt field', () => {
    const result = validatePluginManifest(validManifest({ engine: {} }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'engine.volt')).toBe(true);
  });

  it('rejects empty engine.volt', () => {
    const result = validatePluginManifest(validManifest({ engine: { volt: '' } }));
    expect(result.valid).toBe(false);
  });

  it('rejects non-string engine.volt', () => {
    const result = validatePluginManifest(validManifest({ engine: { volt: 42 } }));
    expect(result.valid).toBe(false);
  });

  it('rejects engine as array', () => {
    const result = validatePluginManifest(validManifest({ engine: [] }));
    expect(result.valid).toBe(false);
  });

  it('rejects missing backend', () => {
    const { backend: _backend, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'backend')).toBe(true);
  });

  it('rejects empty backend', () => {
    const result = validatePluginManifest(validManifest({ backend: '' }));
    expect(result.valid).toBe(false);
  });

  it('rejects backend that is not .js or .mjs', () => {
    const result = validatePluginManifest(validManifest({ backend: './dist/plugin.ts' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'backend')).toBe(true);
  });

  it('rejects non-string backend', () => {
    const result = validatePluginManifest(validManifest({ backend: 42 }));
    expect(result.valid).toBe(false);
  });

  it('rejects contributes as array', () => {
    const result = validatePluginManifest(validManifest({ contributes: [] }));
    expect(result.valid).toBe(false);
  });

  it('rejects contributes.commands as non-array', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: 'not-array' } }),
    );
    expect(result.valid).toBe(false);
  });

  it('rejects command without id', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: [{ title: 'A' }] } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field.includes('id'))).toBe(true);
  });

  it('rejects command without title', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: [{ id: 'a' }] } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field.includes('title'))).toBe(true);
  });

  it('rejects command that is not an object', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: ['not-object'] } }),
    );
    expect(result.valid).toBe(false);
  });

  it('reports multiple errors at once', () => {
    const result = validatePluginManifest({
      id: 42,
      name: '',
      version: 'bad',
      apiVersion: 'wrong',
      engine: null,
      backend: 42,
      capabilities: 'fs',
    });
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThanOrEqual(6);
  });
});
