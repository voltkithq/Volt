import { describe, expect, it } from 'vitest';

import { validatePluginManifest } from '../../utils/plugin-manifest.js';

import { validManifest } from './fixtures.js';

describe('validatePluginManifest root and core fields', () => {
  it('rejects null input', () => {
    const result = validatePluginManifest(null);
    expect(result.valid).toBe(false);
    expect(result.errors[0].field).toBe('(root)');
  });

  it('rejects array input', () => {
    const result = validatePluginManifest([]);
    expect(result.valid).toBe(false);
    expect(result.errors[0].field).toBe('(root)');
  });

  it('rejects string input', () => {
    const result = validatePluginManifest('not an object');
    expect(result.valid).toBe(false);
  });

  it('rejects undefined input', () => {
    const result = validatePluginManifest(undefined);
    expect(result.valid).toBe(false);
  });

  it('rejects missing id', () => {
    const { id: _id, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'id')).toBe(true);
  });

  it('rejects empty id', () => {
    const result = validatePluginManifest(validManifest({ id: '' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'id')).toBe(true);
  });

  it('rejects single-segment id', () => {
    const result = validatePluginManifest(validManifest({ id: 'myplugin' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'id')).toBe(true);
  });

  it('rejects id with uppercase', () => {
    const result = validatePluginManifest(validManifest({ id: 'Acme.Search' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'id')).toBe(true);
  });

  it('rejects id starting with number', () => {
    const result = validatePluginManifest(validManifest({ id: '1acme.search' }));
    expect(result.valid).toBe(false);
  });

  it('rejects id with special characters', () => {
    const result = validatePluginManifest(validManifest({ id: 'acme.my-plugin' }));
    expect(result.valid).toBe(false);
  });

  it('rejects numeric id', () => {
    const result = validatePluginManifest(validManifest({ id: 42 }));
    expect(result.valid).toBe(false);
  });

  it('rejects missing name', () => {
    const { name: _name, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'name')).toBe(true);
  });

  it('rejects whitespace-only name', () => {
    const result = validatePluginManifest(validManifest({ name: '   ' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'name')).toBe(true);
  });

  it('rejects missing version', () => {
    const { version: _version, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'version')).toBe(true);
  });

  it('rejects non-semver version', () => {
    const result = validatePluginManifest(validManifest({ version: 'latest' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'version')).toBe(true);
  });

  it('rejects two-segment version', () => {
    const result = validatePluginManifest(validManifest({ version: '1.0' }));
    expect(result.valid).toBe(false);
  });

  it('rejects missing apiVersion', () => {
    const { apiVersion: _apiVersion, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'apiVersion')).toBe(true);
  });

  it('rejects non-integer apiVersion', () => {
    const result = validatePluginManifest(validManifest({ apiVersion: 1.5 }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.field === 'apiVersion')).toBe(true);
  });

  it('rejects string apiVersion', () => {
    const result = validatePluginManifest(validManifest({ apiVersion: '1' }));
    expect(result.valid).toBe(false);
  });

  it('rejects zero apiVersion', () => {
    const result = validatePluginManifest(validManifest({ apiVersion: 0 }));
    expect(result.valid).toBe(false);
  });

  it('rejects negative apiVersion', () => {
    const result = validatePluginManifest(validManifest({ apiVersion: -1 }));
    expect(result.valid).toBe(false);
  });
});
