import { describe, expect, it } from 'vitest';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, utimesSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { __testOnly } from '../commands/build.js';

describe('build runtime artifact resolution helpers', () => {
  it('infers build platform from known target triples', () => {
    expect(__testOnly.inferBuildPlatform('x86_64-pc-windows-msvc', 'linux')).toBe('win32');
    expect(__testOnly.inferBuildPlatform('aarch64-apple-darwin', 'linux')).toBe('darwin');
    expect(__testOnly.inferBuildPlatform('x86_64-unknown-linux-gnu', 'darwin')).toBe('linux');
  });

  it('maps artifact filenames by target kind and platform', () => {
    expect(__testOnly.artifactFileNameForTarget('volt_runner', 'bin', 'win32')).toBe(
      'volt_runner.exe',
    );
    expect(__testOnly.artifactFileNameForTarget('volt_runner', 'bin', 'linux')).toBe('volt_runner');
    expect(__testOnly.artifactFileNameForTarget('volt_napi', 'cdylib', 'win32')).toBe(
      'volt_napi.dll',
    );
    expect(__testOnly.artifactFileNameForTarget('volt_napi', 'cdylib', 'darwin')).toBe(
      'libvolt_napi.dylib',
    );
    expect(__testOnly.artifactFileNameForTarget('volt_napi', 'cdylib', 'linux')).toBe(
      'libvolt_napi.so',
    );
  });

  it('prioritizes executable artifacts before library artifacts', () => {
    const candidates = __testOnly.collectRuntimeArtifactCandidates(
      [
        { name: 'volt_runner', kind: ['bin'] },
        { name: 'volt_napi', kind: ['cdylib'] },
      ],
      'win32',
    );
    expect(candidates[0]).toMatchObject({
      kind: 'bin',
      targetName: 'volt_runner',
      fileName: 'volt_runner.exe',
    });
    expect(candidates[1]).toMatchObject({
      kind: 'cdylib',
      targetName: 'volt_napi',
      fileName: 'volt_napi.dll',
    });
  });

  it('provides stable fallback candidates when metadata is unavailable', () => {
    const candidates = __testOnly.fallbackRuntimeArtifactCandidates('win32');
    expect(candidates.some((candidate) => candidate.fileName === 'volt-runner.exe')).toBe(true);
    expect(candidates.some((candidate) => candidate.fileName === 'volt_runner.exe')).toBe(true);
  });

  it('resolves the first existing runtime artifact candidate and records attempted paths', () => {
    const candidates = __testOnly.collectRuntimeArtifactCandidates(
      [
        { name: 'volt_runner', kind: ['bin'] },
        { name: 'volt_napi', kind: ['cdylib'] },
      ],
      'win32',
    );

    const releaseDir = join(tmpdir(), 'volt-build-resolve-test');
    const result = __testOnly.selectRuntimeArtifact(candidates, releaseDir, (path) =>
      path.endsWith('volt_napi.dll'),
    );

    expect(result.artifact).toEqual({
      kind: 'cdylib',
      targetName: 'volt_napi',
      sourcePath: join(releaseDir, 'volt_napi.dll'),
    });
    expect(result.attemptedPaths).toEqual([
      join(releaseDir, 'volt_runner.exe'),
      join(releaseDir, 'volt_napi.dll'),
    ]);
  });

  it('returns null artifact with attempted paths when no candidates exist on disk', () => {
    const candidates = __testOnly.collectRuntimeArtifactCandidates(
      [{ name: 'volt_napi', kind: ['cdylib'] }],
      'linux',
    );

    const result = __testOnly.selectRuntimeArtifact(candidates, '/repo/target/release', () => false);
    expect(result.artifact).toBeNull();
    expect(result.attemptedPaths).toHaveLength(1);
    expect(result.attemptedPaths[0]).toMatch(/[\\/]repo[\\/]target[\\/]release[\\/]libvolt_napi\.so$/);
  });

  it('cleans up stale asset bundle path when present', () => {
    const removedPaths: string[] = [];
    const removed = __testOnly.cleanupAssetBundleIfExists(
      '/repo/.volt-assets.bin',
      () => true,
      (path) => {
        removedPaths.push(path);
      },
    );

    expect(removed).toBe(true);
    expect(removedPaths).toEqual(['/repo/.volt-assets.bin']);
  });

  it('does not attempt cleanup when bundle path is missing', () => {
    let removeCalls = 0;
    const removed = __testOnly.cleanupAssetBundleIfExists(
      null,
      () => true,
      () => {
        removeCalls += 1;
      },
    );

    expect(removed).toBe(false);
    expect(removeCalls).toBe(0);
  });

  it('prepares output directory by removing stale files', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-build-outdir-'));
    const outputDir = join(projectDir, 'dist-volt');
    mkdirSync(outputDir, { recursive: true });
    const staleFile = join(outputDir, 'stale.dll');
    writeFileSync(staleFile, 'stale', 'utf8');

    __testOnly.prepareOutputDirectory(outputDir);

    expect(existsSync(outputDir)).toBe(true);
    expect(existsSync(staleFile)).toBe(false);
  });

  it('removes only stale scoped temp directories', () => {
    const root = mkdtempSync(join(tmpdir(), 'volt-build-stale-recovery-'));
    const staleDir = join(root, 'run-stale');
    const freshDir = join(root, 'run-fresh');
    const unrelatedDir = join(root, 'other-dir');
    mkdirSync(staleDir, { recursive: true });
    mkdirSync(freshDir, { recursive: true });
    mkdirSync(unrelatedDir, { recursive: true });

    const nowMs = Date.now();
    const staleTimestamp = new Date(nowMs - 120_000);
    utimesSync(staleDir, staleTimestamp, staleTimestamp);
    const freshTimestamp = new Date(nowMs - 5_000);
    utimesSync(freshDir, freshTimestamp, freshTimestamp);

    const recovery = __testOnly.recoverStaleScopedDirectories(root, {
      prefix: 'run-',
      staleAfterMs: 60_000,
      nowMs,
    });

    expect(recovery).toEqual({
      scanned: 2,
      removed: 1,
      failures: 0,
    });
    expect(existsSync(staleDir)).toBe(false);
    expect(existsSync(freshDir)).toBe(true);
    expect(existsSync(unrelatedDir)).toBe(true);
  });

  it('creates scoped temp directories under the requested root', () => {
    const root = mkdtempSync(join(tmpdir(), 'volt-build-scoped-temp-'));
    const tempDir = __testOnly.createScopedTempDirectory(root, 'run-');
    expect(tempDir.startsWith(root)).toBe(true);
    expect(existsSync(tempDir)).toBe(true);
  });

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

  it('builds runner config payload with permissions, plugin settings, and window options', () => {
    const payload = __testOnly.buildRunnerConfigPayload({
      name: 'IPC Demo',
      devtools: true,
      permissions: ['clipboard'],
      plugins: {
        enabled: ['acme.search'],
        grants: {
          'acme.search': ['fs', 'http'],
        },
        pluginDirs: ['./plugins'],
        limits: {
          activationTimeoutMs: 10_000,
          deactivationTimeoutMs: 5_000,
          callTimeoutMs: 30_000,
          maxPlugins: 32,
        },
        spawning: {
          strategy: 'lazy',
          idleTimeoutMs: 300_000,
          preSpawn: ['acme.search'],
        },
      },
      runtime: {
        poolSize: 3,
      },
      runtimePoolSize: 3,
      window: {
        width: 980,
        height: 760,
        title: 'Volt IPC Demo',
      },
    });

    expect(payload).toMatchObject({
      name: 'IPC Demo',
      devtools: true,
      permissions: ['clipboard'],
      plugins: {
        enabled: ['acme.search'],
        grants: {
          'acme.search': ['fs', 'http'],
        },
        pluginDirs: ['./plugins'],
        limits: {
          activationTimeoutMs: 10_000,
          deactivationTimeoutMs: 5_000,
          callTimeoutMs: 30_000,
          maxPlugins: 32,
        },
        spawning: {
          strategy: 'lazy',
          idleTimeoutMs: 300_000,
          preSpawn: ['acme.search'],
        },
      },
      runtime: {
        poolSize: 3,
      },
      runtimePoolSize: 3,
      window: {
        width: 980,
        height: 760,
        title: 'Volt IPC Demo',
      },
    });
  });
});
