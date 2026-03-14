import { describe, expect, it, vi } from 'vitest';

// Mock isToolAvailable before importing preflight
vi.mock('../utils/signing.js', () => ({
  isToolAvailable: vi.fn(() => true),
}));

import { runBuildPreflight, runPackagePreflight, enforcePreflightResult } from '../utils/preflight.js';
import { isToolAvailable } from '../utils/signing.js';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const mockedIsToolAvailable = vi.mocked(isToolAvailable);

describe('runBuildPreflight', () => {
  it('passes when all tools are available', () => {
    mockedIsToolAvailable.mockReturnValue(true);
    const result = runBuildPreflight('/tmp/test', {}, {});
    expect(result.ok).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it('fails when cargo is missing and no pre-built runner', () => {
    mockedIsToolAvailable.mockImplementation((tool) => tool !== 'cargo');
    const result = runBuildPreflight('/tmp/test', {}, {});
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'build.cargo')).toBe(true);
  });

  it('fails when rustc is missing and no pre-built runner', () => {
    mockedIsToolAvailable.mockImplementation((tool) => tool !== 'rustc');
    const result = runBuildPreflight('/tmp/test', {}, {});
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'build.rustc')).toBe(true);
  });

  it('skips Rust checks when pre-built runner is available', () => {
    mockedIsToolAvailable.mockReturnValue(false);
    const result = runBuildPreflight('/tmp/test', {}, { hasPrebuiltRunner: true });
    expect(result.ok).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it('fails when configured backend entry does not exist', () => {
    mockedIsToolAvailable.mockReturnValue(true);
    const result = runBuildPreflight('/tmp/test', { backend: 'src/nonexistent.ts' }, {});
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'build.backend')).toBe(true);
  });

  it('passes when configured backend entry exists', () => {
    const dir = mkdtempSync(join(tmpdir(), 'preflight-'));
    writeFileSync(join(dir, 'backend.ts'), 'void 0;');
    mockedIsToolAvailable.mockReturnValue(true);
    const result = runBuildPreflight(dir, { backend: 'backend.ts' }, {});
    expect(result.ok).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });
});

describe('runPackagePreflight', () => {
  it('fails when dist-volt does not exist', () => {
    mockedIsToolAvailable.mockReturnValue(true);
    const result = runPackagePreflight('/tmp/nonexistent', 'win32', {
      distVoltDir: '/tmp/nonexistent/dist-volt',
    });
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'package.dist')).toBe(true);
  });

  it('fails when makensis is missing for NSIS format on Windows', () => {
    const dir = mkdtempSync(join(tmpdir(), 'preflight-'));
    const distDir = join(dir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    mockedIsToolAvailable.mockImplementation((tool) => tool !== 'makensis');
    const result = runPackagePreflight(dir, 'win32', {
      format: 'nsis',
      distVoltDir: distDir,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'package.nsis')).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });

  it('fails when appimagetool is missing for AppImage format on Linux', () => {
    const dir = mkdtempSync(join(tmpdir(), 'preflight-'));
    const distDir = join(dir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    mockedIsToolAvailable.mockImplementation((tool) => tool !== 'appimagetool');
    const result = runPackagePreflight(dir, 'linux', {
      format: 'appimage',
      distVoltDir: distDir,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'package.appimage')).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });

  it('fails when dpkg-deb is missing for deb format on Linux', () => {
    const dir = mkdtempSync(join(tmpdir(), 'preflight-'));
    const distDir = join(dir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    mockedIsToolAvailable.mockImplementation((tool) => tool !== 'dpkg-deb');
    const result = runPackagePreflight(dir, 'linux', {
      format: 'deb',
      distVoltDir: distDir,
    });
    expect(result.ok).toBe(false);
    expect(result.errors.some((e) => e.id === 'package.deb')).toBe(true);
    rmSync(dir, { recursive: true, force: true });
  });

  it('passes when all tools are available and dist-volt exists', () => {
    const dir = mkdtempSync(join(tmpdir(), 'preflight-'));
    const distDir = join(dir, 'dist-volt');
    mkdirSync(distDir, { recursive: true });
    mockedIsToolAvailable.mockReturnValue(true);
    const result = runPackagePreflight(dir, 'win32', {
      format: 'nsis',
      distVoltDir: distDir,
    });
    expect(result.ok).toBe(true);
    expect(result.errors).toHaveLength(0);
    rmSync(dir, { recursive: true, force: true });
  });
});

describe('enforcePreflightResult', () => {
  it('does not exit when result is ok', () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation((() => {}) as never);
    enforcePreflightResult({ ok: true, errors: [], warnings: [] });
    expect(exitSpy).not.toHaveBeenCalled();
    exitSpy.mockRestore();
  });

  it('exits with code 1 when result has errors', () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation((() => {}) as never);
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    enforcePreflightResult({
      ok: false,
      errors: [{ id: 'test', message: 'test error', fix: 'fix it' }],
      warnings: [],
    });
    expect(exitSpy).toHaveBeenCalledWith(1);
    exitSpy.mockRestore();
    errorSpy.mockRestore();
  });

  it('prints warnings without exiting', () => {
    const exitSpy = vi.spyOn(process, 'exit').mockImplementation((() => {}) as never);
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    enforcePreflightResult({
      ok: true,
      errors: [],
      warnings: [{ id: 'test', message: 'test warning' }],
    });
    expect(exitSpy).not.toHaveBeenCalled();
    expect(warnSpy).toHaveBeenCalled();
    exitSpy.mockRestore();
    warnSpy.mockRestore();
  });
});
