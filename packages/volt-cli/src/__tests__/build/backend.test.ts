import { describe, expect, it } from 'vitest';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import { __testOnly } from '../../commands/build.js';

describe('build backend helpers', () => {
  it('resolves configured backend entry path', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-'));
    const backendPath = join(projectDir, 'src', 'backend.ts');
    mkdirSync(join(projectDir, 'src'), { recursive: true });
    writeFileSync(backendPath, 'export {};\n', 'utf8');

    const resolved = __testOnly.resolveBackendEntry(projectDir, './src/backend.ts');
    expect(resolved).toBe(backendPath);
  });

  it('returns null when no backend entry is present', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-empty-'));
    const resolved = __testOnly.resolveBackendEntry(projectDir, undefined);
    expect(resolved).toBeNull();
  });

  it('rejects unsupported configured backend entry extension', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-ext-'));
    const backendPath = join(projectDir, 'src', 'backend.json');
    mkdirSync(join(projectDir, 'src'), { recursive: true });
    writeFileSync(backendPath, '{}\n', 'utf8');

    expect(() => __testOnly.resolveBackendEntry(projectDir, './src/backend.json')).toThrow(
      'Unsupported backend entry extension',
    );
  });

  it('writes a safe fallback backend bundle when no backend entry exists', async () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-fallback-'));
    const bundlePath = join(projectDir, 'backend.bundle.mjs');

    await __testOnly.buildBackendBundle(projectDir, null, bundlePath);

    expect(readFileSync(bundlePath, 'utf8').trim()).toBe('void 0;');
  });

  it('builds backend entry and preserves volt:* imports as externals', async () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-esbuild-'));
    const srcDir = join(projectDir, 'src');
    const backendPath = join(srcDir, 'backend.ts');
    const bundlePath = join(projectDir, 'backend.bundle.mjs');
    mkdirSync(srcDir, { recursive: true });
    writeFileSync(
      backendPath,
      [
        "import { sha256 } from 'volt:crypto';",
        'export async function run() {',
        "  return sha256('volt');",
        '}',
      ].join('\n'),
      'utf8',
    );

    await __testOnly.buildBackendBundle(projectDir, backendPath, bundlePath);

    const bundled = readFileSync(bundlePath, 'utf8');
    expect(bundled.length).toBeGreaterThan(0);
    expect(bundled).toContain('volt:crypto');
  });

  it('rejects configured backend paths that escape project root', () => {
    const workspaceDir = mkdtempSync(join(tmpdir(), 'volt-build-backend-scope-'));
    const projectDir = join(workspaceDir, 'app');
    const outsidePath = join(workspaceDir, 'outside.ts');
    mkdirSync(projectDir, { recursive: true });
    writeFileSync(outsidePath, 'export {};\n', 'utf8');

    expect(() => __testOnly.resolveBackendEntry(projectDir, '../outside.ts')).toThrow(
      'must reside within project root',
    );
  });
});
