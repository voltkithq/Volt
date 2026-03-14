import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { isToolAvailable } from './signing.js';

/**
 * Patch the icon of a Windows PE executable after it has been built/patched.
 *
 * Uses `rcedit` if available on PATH, which is the standard tool for editing
 * Windows PE resources (used by Electron, Tauri, etc.). If rcedit is not
 * available, silently skips — the app will run fine without a custom icon.
 *
 * @param exePath - Path to the .exe to patch
 * @param icoPath - Path to the .ico file to embed
 * @param appName - Application name for the file description
 * @param appVersion - Application version string
 * @returns true if the icon was patched, false if skipped
 */
export function patchExeIcon(
  exePath: string,
  icoPath: string,
  appName?: string,
  appVersion?: string,
): boolean {
  if (!existsSync(icoPath)) return false;

  // Try rcedit (standard PE resource editor)
  const rceditTool = findRcedit();
  if (!rceditTool) {
    console.warn(
      '[volt] rcedit not found — skipping exe icon embedding for pre-built runner.\n' +
      '[volt] Install rcedit to embed custom icons: npm install -g rcedit',
    );
    return false;
  }

  try {
    const args = ['--set-icon', icoPath];
    if (appName) {
      args.push('--set-version-string', 'ProductName', appName);
      args.push('--set-version-string', 'FileDescription', appName);
    }
    if (appVersion) {
      args.push('--set-version-string', 'FileVersion', appVersion);
      args.push('--set-version-string', 'ProductVersion', appVersion);
    }
    execFileSync(rceditTool, [exePath, ...args], { stdio: 'pipe' });
    return true;
  } catch (err) {
    console.warn(`[volt] Failed to patch exe icon: ${err}`);
    return false;
  }
}

function findRcedit(): string | null {
  // Check for rcedit on PATH
  if (isToolAvailable('rcedit')) return 'rcedit';
  if (isToolAvailable('rcedit-x64')) return 'rcedit-x64';

  // Check common npm global install locations
  const candidates = [
    'node_modules/.bin/rcedit',
    'node_modules/.bin/rcedit-x64',
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }

  return null;
}
