import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/package.js';
import { readRuntimeArtifactManifest, writeRuntimeArtifactManifest } from '../utils/runtime-artifact.js';
import { createTempDir, createTempDistDir } from './package-artifact-validation.shared.js';

describe('package runtime artifact validation', () => {
  it('uses build manifest when available and rejects non-executable artifacts for windows', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.dll'), 'binary', 'utf8');
    writeFileSync(
      join(distDir, '.volt-runtime-artifact.json'),
      JSON.stringify({
        schemaVersion: 1,
        artifactFileName: 'my-app.dll',
        cargoArtifactKind: 'cdylib',
        cargoTargetName: 'volt_napi',
        rustTarget: 'x86_64-pc-windows-msvc',
      }),
      'utf8',
    );

    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact?.fileName).toBe('my-app.dll');
    expect(resolved.artifact?.runtimeKind).toBe('library');

    const compatibility = __testOnly.validateRuntimeArtifactCompatibility(resolved.artifact!, 'win32');
    expect(compatibility.ok).toBe(false);
    expect(compatibility.reason).toContain('requires an executable runtime artifact (.exe)');
  });

  it('accepts executable artifacts for windows packaging', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeFileSync(
      join(distDir, '.volt-runtime-artifact.json'),
      JSON.stringify({
        schemaVersion: 1,
        artifactFileName: 'my-app.exe',
        cargoArtifactKind: 'bin',
        cargoTargetName: 'volt_runner',
        rustTarget: 'x86_64-pc-windows-msvc',
      }),
      'utf8',
    );

    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    const compatibility = __testOnly.validateRuntimeArtifactCompatibility(resolved.artifact!, 'win32');
    expect(compatibility.ok).toBe(true);
  });

  it('enforces executable-only compatibility for non-windows targets', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.so'), 'binary', 'utf8');

    const resolvedLibrary = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    const linuxCheck = __testOnly.validateRuntimeArtifactCompatibility(resolvedLibrary.artifact!, 'linux');
    expect(linuxCheck.ok).toBe(false);
    expect(linuxCheck.reason).toContain('linux packaging requires an executable runtime artifact');

    writeFileSync(join(distDir, 'my-app'), 'binary', 'utf8');
    const resolvedExecutable = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    const darwinCheck = __testOnly.validateRuntimeArtifactCompatibility(resolvedExecutable.artifact!, 'darwin');
    expect(darwinCheck.ok).toBe(true);
  });

  it('falls back to dist scan when manifest is missing', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');

    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact?.fileName).toBe('my-app.exe');
    expect(resolved.attemptedPaths.length).toBeGreaterThan(0);
  });

  it('returns null when no runtime artifact exists', () => {
    const distDir = createTempDistDir();
    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact).toBeNull();
    expect(resolved.attemptedPaths.length).toBeGreaterThan(0);
  });

  it('round-trips runtime artifact manifests and resolves through manifest metadata', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    const parsedManifest = readRuntimeArtifactManifest(distDir);
    expect(parsedManifest).toEqual({
      schemaVersion: 1,
      artifactFileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-pc-windows-msvc',
    });

    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact).toMatchObject({
      fileName: 'my-app.exe',
      cargoArtifactKind: 'bin',
      runtimeKind: 'executable',
    });
  });

  it('rejects unsafe artifact paths when reading manifests and falls back to dist scan', () => {
    const rootDir = createTempDir('volt-package-root-');
    const distDir = join(rootDir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });

    writeFileSync(join(rootDir, 'outside.exe'), 'binary', 'utf8');
    writeFileSync(join(distDir, 'my-app.exe'), 'binary', 'utf8');
    writeFileSync(
      join(distDir, '.volt-runtime-artifact.json'),
      JSON.stringify({
        schemaVersion: 1,
        artifactFileName: '../outside.exe',
        cargoArtifactKind: 'bin',
        cargoTargetName: 'volt_runner',
        rustTarget: 'x86_64-pc-windows-msvc',
      }),
      'utf8',
    );

    expect(readRuntimeArtifactManifest(distDir)).toBeNull();
    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact?.fileName).toBe('my-app.exe');
  });

  it('rejects writing manifests with unsafe artifact file names', () => {
    const distDir = createTempDistDir();
    expect(() =>
      writeRuntimeArtifactManifest(distDir, {
        schemaVersion: 1,
        artifactFileName: '../outside.exe',
        cargoArtifactKind: 'bin',
        cargoTargetName: 'volt_runner',
        rustTarget: 'x86_64-pc-windows-msvc',
      }),
    ).toThrow('Invalid runtime artifact file name');
  });

  it('treats inconsistent manifest/file combinations as library artifacts', () => {
    const distDir = createTempDistDir();
    writeFileSync(join(distDir, 'my-app.so'), 'binary', 'utf8');
    writeRuntimeArtifactManifest(distDir, {
      schemaVersion: 1,
      artifactFileName: 'my-app.so',
      cargoArtifactKind: 'bin',
      cargoTargetName: 'volt_runner',
      rustTarget: 'x86_64-unknown-linux-gnu',
    });

    const resolved = __testOnly.resolveRuntimeArtifactForPackaging(distDir, 'my-app');
    expect(resolved.artifact).toMatchObject({
      fileName: 'my-app.so',
      cargoArtifactKind: 'bin',
      runtimeKind: 'library',
    });
  });

  it('normalizes package platform inputs from both aliases and triples', () => {
    expect(__testOnly.normalizePackagePlatform('win32', 'linux')).toBe('win32');
    expect(__testOnly.normalizePackagePlatform('x86_64-pc-windows-msvc', 'linux')).toBe('win32');
    expect(__testOnly.normalizePackagePlatform('darwin', 'linux')).toBe('darwin');
    expect(__testOnly.normalizePackagePlatform('aarch64-apple-darwin', 'linux')).toBe('darwin');
    expect(__testOnly.normalizePackagePlatform('x86_64-unknown-linux-gnu', 'win32')).toBe('linux');
  });

  it('maps rust targets to debian architecture values', () => {
    expect(__testOnly.inferDebArchitecture('x86_64-unknown-linux-gnu', null, 'arm64')).toBe('amd64');
    expect(__testOnly.inferDebArchitecture(undefined, 'aarch64-unknown-linux-gnu', 'x64')).toBe('arm64');
    expect(__testOnly.inferDebArchitecture(undefined, 'armv7-unknown-linux-gnueabihf', 'x64')).toBe('armhf');
    expect(__testOnly.inferDebArchitecture(undefined, 'arm-unknown-linux-gnueabihf', 'x64')).toBe('armhf');
    expect(__testOnly.inferDebArchitecture(undefined, 'arm-unknown-linux-gnueabi', 'x64')).toBe('armel');
    expect(__testOnly.inferDebArchitecture(undefined, null, 'ia32')).toBe('i386');
  });

  it('maps linux targets to AppImage architecture suffixes', () => {
    expect(__testOnly.inferAppImageArchitecture('x86_64-unknown-linux-gnu', null, 'arm64')).toBe('x86_64');
    expect(__testOnly.inferAppImageArchitecture(undefined, 'aarch64-unknown-linux-gnu', 'x64')).toBe('aarch64');
    expect(__testOnly.inferAppImageArchitecture(undefined, 'armv7-unknown-linux-gnueabihf', 'x64')).toBe('armhf');
    expect(__testOnly.inferAppImageArchitecture(undefined, 'arm-unknown-linux-gnueabihf', 'x64')).toBe('armhf');
    expect(__testOnly.inferAppImageArchitecture(undefined, 'arm-unknown-linux-gnueabi', 'x64')).toBe('armhf');
    expect(__testOnly.inferAppImageArchitecture(undefined, null, 'ia32')).toBe('i686');
  });
});
