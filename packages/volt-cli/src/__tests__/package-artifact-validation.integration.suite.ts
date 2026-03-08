import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { writeRuntimeArtifactManifest } from '../utils/runtime-artifact.js';
import { createTempProjectDir, runPackageCommand } from './package-artifact-validation.shared.js';

describe('package command integration: runtime artifact validation', () => {
  it('fails fast when build output contains non-executable runtime artifact for target platform', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app.dll'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.dll',
      cargoArtifactKind: 'cdylib',
      cargoTargetName: 'volt_napi',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    const result = await runPackageCommand(projectDir, { target: 'win32', format: 'nsis' });
    expect(result.exitCode).toBe(1);
    expect(result.errors.some((line) => line.includes('requires an executable runtime artifact (.exe)'))).toBe(true);
  });

  it('accepts executable runtime artifacts and proceeds into packaging flow', async () => {
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

    const result = await runPackageCommand(projectDir, { target: 'win32', format: 'nsis' });
    expect(result.exitCode).toBeNull();
    expect(result.logs.some((line) => line.includes('Runtime artifact: my-app.exe'))).toBe(true);
  });

  it('fails fast for linux packaging when runtime artifact is a library', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app.so'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.so',
      cargoArtifactKind: 'cdylib',
      cargoTargetName: 'volt_napi',
      rustTarget: 'x86_64-unknown-linux-gnu',
    });

    const result = await runPackageCommand(projectDir, { target: 'linux', format: 'appimage' });
    expect(result.exitCode).toBe(1);
    expect(result.errors.some((line) => line.includes('linux packaging requires an executable runtime artifact'))).toBe(
      true,
    );
  });

  it('uses fallback artifact scan on linux when manifest is missing', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app'), 'binary', 'utf8');

    const result = await runPackageCommand(projectDir, { target: 'linux', format: 'appimage' });
    expect(result.exitCode).toBeNull();
    expect(result.logs.some((line) => line.includes('Runtime artifact: my-app'))).toBe(true);
  });

  it('rejects linux packaging when manifest claims executable but file is a library', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    writeFileSync(join(distDir, 'my-app.so'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.so',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-unknown-linux-gnu',
    });

    const result = await runPackageCommand(projectDir, { target: 'linux', format: 'appimage' });
    expect(result.exitCode).toBe(1);
    expect(result.errors.some((line) => line.includes('linux packaging requires an executable runtime artifact'))).toBe(
      true,
    );
  });

  it('fails fast when requested package format is unsupported for the selected platform', async () => {
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

    const result = await runPackageCommand(projectDir, { target: 'win32', format: 'deb' });
    expect(result.exitCode).toBe(1);
    expect(result.errors.some((line) => line.includes('Unsupported package format'))).toBe(true);
  });

  it('supports MSIX packaging mode and generates enterprise deployment outputs', async () => {
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

    const result = await runPackageCommand(projectDir, { target: 'win32', format: 'msix' });
    expect(result.exitCode).toBeNull();
    expect(result.logs.some((line) => line.includes('Creating Windows MSIX package'))).toBe(true);
    expect(existsSync(join(projectDir, 'dist-package', 'my-app-msix-staging', 'AppxManifest.xml'))).toBe(true);
    expect(existsSync(join(projectDir, 'dist-package', 'enterprise', 'policy', 'Volt.admx'))).toBe(true);
    expect(existsSync(join(projectDir, 'dist-package', 'enterprise', 'DEPLOYMENT.md'))).toBe(true);
  });

  it('generates a full enterprise bundle roundtrip with policy and deployment scripts', async () => {
    const projectDir = createTempProjectDir();
    const distDir = join(projectDir, 'dist-volt');
    const packageDir = join(projectDir, 'dist-package');
    mkdirSync(distDir, { recursive: true });
    mkdirSync(packageDir, { recursive: true });

    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    // Seed expected installer names so enterprise scripts/readme can reference concrete artifacts
    // even when packaging tools are unavailable in CI.
    writeFileSync(join(packageDir, 'my-app-0.1.0-setup.exe'), 'placeholder', 'utf8');
    writeFileSync(join(packageDir, 'my-app-0.1.0.msix'), 'placeholder', 'utf8');

    const result = await runPackageCommand(projectDir, {
      target: 'win32',
      format: 'nsis',
      installMode: 'perUser',
    });
    expect(result.exitCode).toBeNull();

    const admxPath = join(packageDir, 'enterprise', 'policy', 'Volt.admx');
    const admlPath = join(packageDir, 'enterprise', 'policy', 'en-US', 'Volt.adml');
    const policyValuesPath = join(packageDir, 'enterprise', 'policy', 'policy-values.json');
    const readmePath = join(packageDir, 'enterprise', 'DEPLOYMENT.md');
    const nsisMachineScriptPath = join(packageDir, 'enterprise', 'scripts', 'install-nsis-allusers.ps1');
    const nsisUserScriptPath = join(packageDir, 'enterprise', 'scripts', 'install-nsis-current-user.ps1');
    const msixScriptPath = join(packageDir, 'enterprise', 'scripts', 'install-msix.ps1');

    expect(existsSync(admxPath)).toBe(true);
    expect(existsSync(admlPath)).toBe(true);
    expect(existsSync(policyValuesPath)).toBe(true);
    expect(existsSync(readmePath)).toBe(true);
    expect(existsSync(nsisMachineScriptPath)).toBe(true);
    expect(existsSync(nsisUserScriptPath)).toBe(true);
    expect(existsSync(msixScriptPath)).toBe(true);

    const policyValues = JSON.parse(readFileSync(policyValuesPath, 'utf8')) as Record<string, unknown>;
    expect(policyValues['InstallMode']).toBe('perUser');

    const readme = readFileSync(readmePath, 'utf8');
    expect(readme).toContain('my-app-0.1.0-setup.exe');
    expect(readme).toContain('my-app-0.1.0.msix');

    const allUsersScript = readFileSync(nsisMachineScriptPath, 'utf8');
    const currentUserScript = readFileSync(nsisUserScriptPath, 'utf8');
    const msixScript = readFileSync(msixScriptPath, 'utf8');
    expect(allUsersScript).toContain('/ALLUSERS=1');
    expect(currentUserScript).toContain('/ALLUSERS=0');
    expect(msixScript).toContain('my-app-0.1.0.msix');
  });
});
