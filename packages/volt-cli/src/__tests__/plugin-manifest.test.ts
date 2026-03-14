import { describe, it, expect } from 'vitest';
import { validatePluginManifest } from '../utils/plugin-manifest.js';

function validManifest(overrides?: Record<string, unknown>) {
  return {
    id: 'acme.notes.search',
    name: 'Notes Search',
    version: '0.1.0',
    apiVersion: 1,
    engine: { volt: '>=0.2.0' },
    backend: './dist/plugin.js',
    capabilities: ['http', 'fs'],
    ...overrides,
  };
}

describe('validatePluginManifest', () => {
  // ── Valid manifests ────────────────────────────────────────────

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
    const result = validatePluginManifest(
      validManifest({ engine: { volt: '~0.2.0' } }),
    );
    expect(result.valid).toBe(true);
  });

  it('accepts semver range with caret', () => {
    const result = validatePluginManifest(
      validManifest({ engine: { volt: '^0.2.0' } }),
    );
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

  // ── Invalid root ───────────────────────────────────────────────

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

  // ── Invalid id ─────────────────────────────────────────────────

  it('rejects missing id', () => {
    const { id: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'id')).toBe(true);
  });

  it('rejects empty id', () => {
    const result = validatePluginManifest(validManifest({ id: '' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'id')).toBe(true);
  });

  it('rejects single-segment id', () => {
    const result = validatePluginManifest(validManifest({ id: 'myplugin' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'id')).toBe(true);
  });

  it('rejects id with uppercase', () => {
    const result = validatePluginManifest(validManifest({ id: 'Acme.Search' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'id')).toBe(true);
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

  // ── Invalid name ───────────────────────────────────────────────

  it('rejects missing name', () => {
    const { name: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'name')).toBe(true);
  });

  it('rejects whitespace-only name', () => {
    const result = validatePluginManifest(validManifest({ name: '   ' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'name')).toBe(true);
  });

  // ── Invalid version ────────────────────────────────────────────

  it('rejects missing version', () => {
    const { version: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'version')).toBe(true);
  });

  it('rejects non-semver version', () => {
    const result = validatePluginManifest(validManifest({ version: 'latest' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'version')).toBe(true);
  });

  it('rejects two-segment version', () => {
    const result = validatePluginManifest(validManifest({ version: '1.0' }));
    expect(result.valid).toBe(false);
  });

  // ── Invalid apiVersion ─────────────────────────────────────────

  it('rejects missing apiVersion', () => {
    const { apiVersion: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'apiVersion')).toBe(true);
  });

  it('rejects non-integer apiVersion', () => {
    const result = validatePluginManifest(validManifest({ apiVersion: 1.5 }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'apiVersion')).toBe(true);
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

  // ── Invalid engine ─────────────────────────────────────────────

  it('rejects missing engine', () => {
    const { engine: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'engine')).toBe(true);
  });

  it('rejects engine without volt field', () => {
    const result = validatePluginManifest(validManifest({ engine: {} }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'engine.volt')).toBe(true);
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

  // ── Invalid backend ────────────────────────────────────────────

  it('rejects missing backend', () => {
    const { backend: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'backend')).toBe(true);
  });

  it('rejects empty backend', () => {
    const result = validatePluginManifest(validManifest({ backend: '' }));
    expect(result.valid).toBe(false);
  });

  it('rejects backend that is not .js or .mjs', () => {
    const result = validatePluginManifest(validManifest({ backend: './dist/plugin.ts' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'backend')).toBe(true);
  });

  it('rejects non-string backend', () => {
    const result = validatePluginManifest(validManifest({ backend: 42 }));
    expect(result.valid).toBe(false);
  });

  // ── Invalid capabilities ───────────────────────────────────────

  it('rejects missing capabilities', () => {
    const { capabilities: _, ...rest } = validManifest();
    const result = validatePluginManifest(rest);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'capabilities')).toBe(true);
  });

  it('rejects capabilities as string', () => {
    const result = validatePluginManifest(validManifest({ capabilities: 'fs' }));
    expect(result.valid).toBe(false);
  });

  it('rejects unknown capability', () => {
    const result = validatePluginManifest(validManifest({ capabilities: ['fs', 'teleport'] }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.message.includes('Unknown capability'))).toBe(true);
  });

  it('rejects non-string element in capabilities', () => {
    const result = validatePluginManifest(validManifest({ capabilities: [42] }));
    expect(result.valid).toBe(false);
  });

  it('rejects duplicate capabilities', () => {
    const result = validatePluginManifest(validManifest({ capabilities: ['fs', 'fs'] }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.message.includes('Duplicate'))).toBe(true);
  });

  // ── Invalid contributes ────────────────────────────────────────

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
    expect(result.errors.some((e) => e.field.includes('id'))).toBe(true);
  });

  it('rejects command without title', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: [{ id: 'a' }] } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field.includes('title'))).toBe(true);
  });

  it('rejects command that is not an object', () => {
    const result = validatePluginManifest(
      validManifest({ contributes: { commands: ['not-object'] } }),
    );
    expect(result.valid).toBe(false);
  });

  // ── Multiple errors ────────────────────────────────────────────

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

  // ── Signature validation ─────────────────────────────────────

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

  it('accepts manifest without signature (still valid)', () => {
    const result = validatePluginManifest(validManifest());
    expect(result.valid).toBe(true);
    expect(result.manifest!.signature).toBeUndefined();
  });

  it('rejects signature as non-object', () => {
    const result = validatePluginManifest(validManifest({ signature: 'bad' }));
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'signature')).toBe(true);
  });

  it('rejects signature with missing algorithm', () => {
    const result = validatePluginManifest(
      validManifest({ signature: { value: 'abc123' } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'signature.algorithm')).toBe(true);
  });

  it('rejects signature with missing value', () => {
    const result = validatePluginManifest(
      validManifest({ signature: { algorithm: 'ed25519' } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'signature.value')).toBe(true);
  });

  it('rejects signature with empty algorithm', () => {
    const result = validatePluginManifest(
      validManifest({ signature: { algorithm: '', value: 'abc' } }),
    );
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.field === 'signature.algorithm')).toBe(true);
  });

  // ── Extra fields are tolerated ─────────────────────────────────

  it('tolerates extra top-level fields', () => {
    const result = validatePluginManifest(validManifest({ description: 'Some plugin' }));
    expect(result.valid).toBe(true);
  });
});
