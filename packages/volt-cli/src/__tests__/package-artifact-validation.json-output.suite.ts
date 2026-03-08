import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { writeRuntimeArtifactManifest } from '../utils/runtime-artifact.js';
import { createTempProjectDir, runPackageCommand } from './package-artifact-validation.shared.js';

describe('package command JSON summary output', () => {
  it('writes machine-readable summary JSON to --json-output path', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    const result = await runPackageCommand(projectDir, {
      target: 'win32',
      format: 'nsis',
      jsonOutput: 'ci/package-summary.json',
    });

    expect(result.exitCode).toBeNull();
    const summaryPath = join(projectDir, 'ci', 'package-summary.json');
    expect(existsSync(summaryPath)).toBe(true);

    const summary = JSON.parse(readFileSync(summaryPath, 'utf8')) as {
      platform: string;
      runtimeArtifact: string;
      installMode: string | null;
      artifacts: unknown[];
      signingResults: unknown[];
    };
    expect(summary.platform).toBe('win32');
    expect(summary.runtimeArtifact).toBe('my-app.exe');
    expect(summary.installMode).toBe('perMachine');
    expect(Array.isArray(summary.artifacts)).toBe(true);
    expect(Array.isArray(summary.signingResults)).toBe(true);
  });

  it('prints machine-readable summary JSON when --json is enabled', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    const result = await runPackageCommand(projectDir, {
      target: 'win32',
      format: 'nsis',
      json: true,
    });

    expect(result.exitCode).toBeNull();
    const jsonLog = result.logs.find((entry) => entry.trimStart().startsWith('{') && entry.includes('"runtimeArtifact"'));
    expect(jsonLog).toBeDefined();

    const summary = JSON.parse(jsonLog ?? '{}') as {
      platform: string;
      runtimeArtifact: string;
      installMode: string | null;
      signingResults: unknown[];
    };
    expect(summary.platform).toBe('win32');
    expect(summary.runtimeArtifact).toBe('my-app.exe');
    expect(summary.installMode).toBe('perMachine');
    expect(Array.isArray(summary.signingResults)).toBe(true);
  });
});
