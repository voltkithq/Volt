import type { ChildProcess } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { __testOnly, VoltAppLauncher } from './launcher.js';

describe('launcher helpers', () => {
  it('resolves runtime binary from manifest', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-test-launcher-'));
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(
      join(distDir, '.volt-runtime-artifact.json'),
      JSON.stringify({ artifactFileName: 'app.exe' }),
      'utf8',
    );
    const binaryPath = join(distDir, 'app.exe');
    writeFileSync(binaryPath, 'binary', 'utf8');

    const resolved = __testOnly.resolveRuntimeBinary(projectDir);
    expect(resolved).toBe(binaryPath);
  });

  it('throws when manifest is missing', () => {
    const projectDir = mkdtempSync(join(tmpdir(), 'volt-test-launcher-missing-'));
    expect(() => __testOnly.resolveRuntimeBinary(projectDir)).toThrow('runtime manifest missing');
  });

  it('runs a scenario end-to-end with prepare/build/launch hooks', async () => {
    const repoRoot = mkdtempSync(join(tmpdir(), 'volt-test-launcher-run-'));
    const sourceProjectDir = join(repoRoot, 'demo-app');
    const sourceSrcDir = join(sourceProjectDir, 'src');
    const cliEntryPath = join(repoRoot, 'fake-cli.mjs');
    mkdirSync(sourceSrcDir, { recursive: true });
    writeFileSync(cliEntryPath, 'export {};\n', 'utf8');
    writeFileSync(join(sourceProjectDir, 'package.json'), '{"name":"demo-app"}\n', 'utf8');

    const launcher = new VoltAppLauncher({
      repoRoot,
      cliEntryPath,
      logger: {
        log: () => {},
        warn: () => {},
        error: () => {},
      },
    });

    const buildSpy = vi.fn((projectDir: string) => {
      const distDir = join(projectDir, 'dist-volt');
      mkdirSync(distDir, { recursive: true });
      writeFileSync(
        join(distDir, '.volt-runtime-artifact.json'),
        JSON.stringify({ artifactFileName: 'app.exe' }),
        'utf8',
      );
      writeFileSync(join(distDir, 'app.exe'), 'binary', 'utf8');
    });

    const launchSpy = vi.fn((_: string, cwd: string) => {
      writeFileSync(join(cwd, '.result.json'), JSON.stringify({ ok: true, value: 42 }), 'utf8');
      const child = {
        exitCode: 0,
        signalCode: null,
        kill: vi.fn(),
        once: vi.fn(),
      };
      return child as unknown as ChildProcess;
    });

    const launcherWithOverrides = launcher as unknown as {
      buildProject: (projectDir: string) => void;
      launchBinary: (binaryPath: string, cwd: string) => ChildProcess;
    };
    launcherWithOverrides.buildProject = buildSpy;
    launcherWithOverrides.launchBinary = launchSpy;

    const prepareSpy = vi.fn((copiedProjectDir: string) => {
      writeFileSync(join(copiedProjectDir, 'prepared.txt'), 'ready', 'utf8');
    });

    const payload = await launcher.run<{ ok: boolean; value: number }>({
      sourceProjectDir,
      resultFile: '.result.json',
      timeoutMs: 2_000,
      prepareProject: prepareSpy,
      validatePayload: (raw) => {
        const value = raw as { ok?: unknown; value?: unknown };
        if (value.ok !== true || typeof value.value !== 'number') {
          throw new Error('invalid payload');
        }
        return {
          ok: true,
          value: value.value,
        };
      },
    });

    expect(payload).toEqual({ ok: true, value: 42 });
    expect(prepareSpy).toHaveBeenCalledTimes(1);
    expect(buildSpy).toHaveBeenCalledTimes(1);
    expect(launchSpy).toHaveBeenCalledTimes(1);
  });

  it('writes result payload artifact when artifactsDir is set', async () => {
    const repoRoot = mkdtempSync(join(tmpdir(), 'volt-test-launcher-artifacts-'));
    const sourceProjectDir = join(repoRoot, 'demo-app');
    const sourceSrcDir = join(sourceProjectDir, 'src');
    const cliEntryPath = join(repoRoot, 'fake-cli.mjs');
    const artifactsDir = join(repoRoot, 'artifacts', 'suite-a');
    mkdirSync(sourceSrcDir, { recursive: true });
    writeFileSync(cliEntryPath, 'export {};\n', 'utf8');
    writeFileSync(join(sourceProjectDir, 'package.json'), '{"name":"demo-app"}\n', 'utf8');

    const launcher = new VoltAppLauncher({
      repoRoot,
      cliEntryPath,
      logger: {
        log: () => {},
        warn: () => {},
        error: () => {},
      },
    });

    const launcherWithOverrides = launcher as unknown as {
      buildProject: (projectDir: string) => void;
      launchBinary: (binaryPath: string, cwd: string) => ChildProcess;
    };

    launcherWithOverrides.buildProject = (projectDir: string) => {
      const distDir = join(projectDir, 'dist-volt');
      mkdirSync(distDir, { recursive: true });
      writeFileSync(
        join(distDir, '.volt-runtime-artifact.json'),
        JSON.stringify({ artifactFileName: 'app.exe' }),
        'utf8',
      );
      writeFileSync(join(distDir, 'app.exe'), 'binary', 'utf8');
    };

    launcherWithOverrides.launchBinary = (_: string, cwd: string) => {
      writeFileSync(join(cwd, '.result.json'), JSON.stringify({ ok: true, value: 7 }), 'utf8');
      return {
        exitCode: 0,
        signalCode: null,
        kill: vi.fn(),
        once: vi.fn(),
      } as unknown as ChildProcess;
    };

    await launcher.run({
      sourceProjectDir,
      resultFile: '.result.json',
      timeoutMs: 2_000,
      artifactsDir,
    });

    const artifactPath = join(artifactsDir, 'result-payload.json');
    expect(existsSync(artifactPath)).toBe(true);
    expect(readFileSync(artifactPath, 'utf8')).toContain('"value": 7');
  });
});
