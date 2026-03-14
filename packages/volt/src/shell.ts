/**
 * Shell module.
 * Provides secure URL opening with protocol validation.
 * Requires `permissions: ['shell']` in volt.config.ts.
 */

import { shellOpenExternal, shellShowItemInFolder } from '@voltkit/volt-native';

/**
 * Open a URL in the default system application.
 * SECURITY: Only allows http, https, and mailto protocols.
 * All other protocols (file://, javascript:, data:, etc.) are rejected.
 *
 * @example
 * ```ts
 * await shell.openExternal('https://example.com');
 * ```
 */
async function openExternal(url: string): Promise<void> {
  // Validate URL scheme on the TypeScript side as well (defense-in-depth)
  const ALLOWED_SCHEMES = ['http:', 'https:', 'mailto:'];
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(`Invalid URL: ${url}`);
  }

  if (!ALLOWED_SCHEMES.includes(parsed.protocol)) {
    throw new Error(
      `Protocol not allowed: '${parsed.protocol}'. Only http, https, and mailto are permitted.`,
    );
  }

  shellOpenExternal(url);
}

/**
 * Reveal a file or directory in the platform file manager.
 * Opens Explorer (Windows), Finder (macOS), or the default file manager (Linux).
 *
 * @example
 * ```ts
 * shell.showItemInFolder('/path/to/file.txt');
 * ```
 */
function showItemInFolder(path: string): void {
  shellShowItemInFolder(path);
}

/** Shell APIs. Requires `permissions: ['shell']` in volt.config.ts. */
export const shell = {
  openExternal,
  showItemInFolder,
};
