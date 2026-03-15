import { describe, expect, it } from 'vitest';

import { validatePluginManifest } from '../../utils/plugin-manifest.js';

import { validManifest } from './fixtures.js';

describe('validatePluginManifest valid manifests', () => {
  it('accepts a minimal valid manifest', () => {
    const result = validatePluginManifest(validManifest());
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
    expect(result.manifest).toBeDefined();
    expect(result.manifest!.id).toBe('acme.notes.search');
  });

  it('accepts a manifest with contributes.commands', () => {
    const result = validatePluginManifest(
      validManifest({
        contributes: {
          commands: [{ id: 'search.reindex', title: 'Reindex Search' }],
        },
      }),
    );
    expect(result.valid).toBe(true);
  });

  it('accepts empty capabilities array', () => {
    const result = validatePluginManifest(validManifest({ capabilities: [] }));
    expect(result.valid).toBe(true);
  });

  it('accepts all known capabilities', () => {
    const result = validatePluginManifest(
      validManifest({
        capabilities: [
          'clipboard',
          'notification',
          'dialog',
          'fs',
          'db',
          'menu',
          'shell',
          'http',
          'globalShortcut',
          'tray',
          'secureStorage',
        ],
      }),
    );
    expect(result.valid).toBe(true);
  });

  it('accepts .mjs backend entry', () => {
    const result = validatePluginManifest(validManifest({ backend: './dist/plugin.mjs' }));
    expect(result.valid).toBe(true);
  });

  it('accepts prerelease semver version', () => {
    const result = validatePluginManifest(validManifest({ version: '1.0.0-beta.1' }));
    expect(result.valid).toBe(true);
  });

  it('accepts version with build metadata', () => {
    const result = validatePluginManifest(validManifest({ version: '1.0.0+build.123' }));
    expect(result.valid).toBe(true);
  });

  it('accepts semver range with tilde', () => {
    const result = validatePluginManifest(validManifest({ engine: { volt: '~0.2.0' } }));
    expect(result.valid).toBe(true);
  });

  it('accepts semver range with caret', () => {
    const result = validatePluginManifest(validManifest({ engine: { volt: '^0.2.0' } }));
    expect(result.valid).toBe(true);
  });

  it('accepts contributes without commands', () => {
    const result = validatePluginManifest(validManifest({ contributes: {} }));
    expect(result.valid).toBe(true);
  });

  it('accepts deep reverse-domain id with many segments', () => {
    const result = validatePluginManifest(validManifest({ id: 'com.example.app.ext.search' }));
    expect(result.valid).toBe(true);
  });

  it('tolerates extra top-level fields', () => {
    const result = validatePluginManifest(validManifest({ description: 'Some plugin' }));
    expect(result.valid).toBe(true);
  });
});
