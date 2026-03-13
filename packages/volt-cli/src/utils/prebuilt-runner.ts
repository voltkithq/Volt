import { createWriteStream, existsSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { get as httpsGet } from 'node:https';
import { get as httpGet } from 'node:http';
import { pipeline } from 'node:stream/promises';

/**
 * Resolve the pre-built runner binary for the current platform/arch.
 * Returns the local path if cached, or downloads it first.
 *
 * The runner version must match the volt-cli version to ensure compatibility.
 */
export async function resolvePrebuiltRunner(options: {
  version: string;
  platform: string;
  arch: string;
  cacheDir: string;
}): Promise<string | null> {
  const target = resolveRustTarget(options.platform, options.arch);
  if (!target) return null;

  const ext = options.platform === 'win32' ? '.exe' : '';
  const fileName = `volt-runner-${options.version}-${target}${ext}`;
  const cachedPath = resolve(options.cacheDir, fileName);

  if (existsSync(cachedPath)) {
    return cachedPath;
  }

  const downloadUrl = buildDownloadUrl(options.version, target, ext);
  if (!downloadUrl) return null;

  try {
    mkdirSync(options.cacheDir, { recursive: true });
    await downloadFile(downloadUrl, cachedPath);
    return cachedPath;
  } catch {
    return null;
  }
}

function resolveRustTarget(platform: string, arch: string): string | null {
  const targets: Record<string, Record<string, string>> = {
    win32: {
      x64: 'x86_64-pc-windows-msvc',
      arm64: 'aarch64-pc-windows-msvc',
    },
    darwin: {
      x64: 'x86_64-apple-darwin',
      arm64: 'aarch64-apple-darwin',
    },
    linux: {
      x64: 'x86_64-unknown-linux-gnu',
      arm64: 'aarch64-unknown-linux-gnu',
    },
  };
  return targets[platform]?.[arch] ?? null;
}

function buildDownloadUrl(version: string, target: string, ext: string): string | null {
  // GitHub release asset URL pattern
  return `https://github.com/voltkithq/Volt/releases/download/v${version}/volt-runner-${target}${ext}`;
}

async function downloadFile(url: string, destPath: string): Promise<void> {
  const maxRedirects = 5;
  let currentUrl = url;

  for (let i = 0; i < maxRedirects; i++) {
    const finalUrl = await new Promise<string | null>((resolveUrl, reject) => {
      const getter = currentUrl.startsWith('https') ? httpsGet : httpGet;
      const req = getter(currentUrl, (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          resolveUrl(res.headers.location);
          res.resume();
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`Download failed: HTTP ${res.statusCode}`));
          return;
        }
        const file = createWriteStream(destPath);
        pipeline(res, file).then(() => resolveUrl(null)).catch(reject);
      });
      req.on('error', reject);
    });

    if (finalUrl === null) return; // download complete
    currentUrl = finalUrl;
  }

  throw new Error('Too many redirects');
}
