import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { isToolAvailable } from './signing.js';

export interface PreflightError {
  id: string;
  message: string;
  fix?: string;
}

export interface PreflightWarning {
  id: string;
  message: string;
}

export interface PreflightResult {
  ok: boolean;
  errors: PreflightError[];
  warnings: PreflightWarning[];
}

/**
 * Validate prerequisites before starting a `volt build`.
 * These checks are fast (file existence, PATH lookups) and run before any
 * expensive operations like Vite builds or Cargo compilations.
 */
export function runBuildPreflight(
  cwd: string,
  config: {
    backend?: string;
    build?: { outDir?: string };
  },
  options: {
    hasPrebuiltRunner?: boolean;
    target?: string;
  },
): PreflightResult {
  const errors: PreflightError[] = [];
  const warnings: PreflightWarning[] = [];

  // Rust toolchain is required unless a pre-built runner is available
  if (!options.hasPrebuiltRunner) {
    if (!isToolAvailable('cargo')) {
      errors.push({
        id: 'build.cargo',
        message: '`cargo` not found on PATH.',
        fix: 'Install Rust: https://rustup.rs',
      });
    }
    if (!isToolAvailable('rustc')) {
      errors.push({
        id: 'build.rustc',
        message: '`rustc` not found on PATH.',
        fix: 'Install Rust: https://rustup.rs',
      });
    }

    // Cross-compilation target check
    if (options.target && isToolAvailable('rustup')) {
      try {
        const output = execFileSync('rustup', ['target', 'list', '--installed'], {
          stdio: 'pipe',
          encoding: 'utf8',
        }) as string;
        const installed = output.split('\n').map((l) => l.trim()).filter(Boolean);
        if (!installed.includes(options.target)) {
          errors.push({
            id: 'build.target',
            message: `Rust target \`${options.target}\` is not installed.`,
            fix: `Run: rustup target add ${options.target}`,
          });
        }
      } catch {
        // Can't check — skip silently
      }
    }
  }

  // Backend entry exists if configured
  if (config.backend && config.backend.trim().length > 0) {
    const backendPath = resolve(cwd, config.backend);
    if (!existsSync(backendPath)) {
      errors.push({
        id: 'build.backend',
        message: `Backend entry not found: ${config.backend}`,
        fix: `Create the file at ${backendPath} or update \`backend\` in volt.config.ts`,
      });
    }
  }

  return { ok: errors.length === 0, errors, warnings };
}

/**
 * Validate prerequisites before starting `volt package`.
 * Catches missing packaging tools before any work begins.
 */
export function runPackagePreflight(
  cwd: string,
  platform: 'win32' | 'darwin' | 'linux',
  options: {
    format?: string;
    distVoltDir?: string;
  },
): PreflightResult {
  const errors: PreflightError[] = [];
  const warnings: PreflightWarning[] = [];

  // dist-volt must exist with build output
  const distVoltDir = options.distVoltDir ?? resolve(cwd, 'dist-volt');
  if (!existsSync(distVoltDir)) {
    errors.push({
      id: 'package.dist',
      message: 'No build output found in dist-volt/.',
      fix: 'Run `volt build` first.',
    });
  }

  // Platform-specific tool checks — these are warnings, not errors.
  // The packager handles missing tools gracefully (skips that format and reports at the end).
  const formats = resolveFormats(platform, options.format);

  if (platform === 'win32') {
    if (formats.includes('nsis') && !isToolAvailable('makensis')) {
      warnings.push({
        id: 'package.nsis',
        message: '`makensis` not found on PATH. NSIS installer will be skipped. Install: choco install nsis',
      });
    }
    if (formats.includes('msix') && !isToolAvailable('makemsix') && !isToolAvailable('makeappx')) {
      warnings.push({
        id: 'package.msix',
        message: '`makemsix`/`makeappx` not found. MSIX package will be skipped. Install Windows SDK.',
      });
    }
  }

  if (platform === 'darwin') {
    if (formats.includes('dmg') && !isToolAvailable('hdiutil')) {
      warnings.push({
        id: 'package.dmg',
        message: '`hdiutil` not found. DMG creation will be skipped.',
      });
    }
  }

  if (platform === 'linux') {
    if (formats.includes('appimage') && !isToolAvailable('appimagetool')) {
      warnings.push({
        id: 'package.appimage',
        message: '`appimagetool` not found. AppImage will be skipped. Download from https://github.com/AppImage/appimagetool/releases',
      });
    }
    if (formats.includes('deb') && !isToolAvailable('dpkg-deb')) {
      warnings.push({
        id: 'package.deb',
        message: '`dpkg-deb` not found. .deb package will be skipped. Install: apt install dpkg',
      });
    }
  }

  return { ok: errors.length === 0, errors, warnings };
}

function resolveFormats(platform: 'win32' | 'darwin' | 'linux', requested?: string): string[] {
  if (requested) return [requested];
  if (platform === 'win32') return ['nsis'];
  if (platform === 'darwin') return ['app'];
  return ['appimage', 'deb'];
}

/**
 * Print preflight results and exit if there are errors.
 */
export function enforcePreflightResult(result: PreflightResult): void {
  for (const warning of result.warnings) {
    console.warn(`[volt] Warning: ${warning.message}`);
  }
  if (!result.ok) {
    console.error('[volt] Pre-flight check failed:');
    for (const error of result.errors) {
      console.error(`[volt]   ✗ ${error.message}`);
      if (error.fix) {
        console.error(`[volt]     Fix: ${error.fix}`);
      }
    }
    process.exit(1);
  }
}
