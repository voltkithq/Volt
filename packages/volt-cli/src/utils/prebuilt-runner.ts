import { chmodSync, createWriteStream, existsSync, mkdirSync, readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { get as httpsGet } from 'node:https';
import { get as httpGet } from 'node:http';
import { createHash } from 'node:crypto';
import { pipeline } from 'node:stream/promises';
import { fileURLToPath } from 'node:url';

/**
 * Read the Volt CLI package version from its own package.json.
 * This is the version used to find matching pre-built runner binaries.
 */
function getVoltCliVersion(): string {
  const cliDir = dirname(fileURLToPath(import.meta.url));
  // From dist/utils/ we go up 2 levels to the package root
  const pkgPath = resolve(cliDir, '..', '..', 'package.json');
  try {
    const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
    return pkg.version;
  } catch {
    return '0.0.0';
  }
}

/**
 * Resolve the pre-built runner binary for the current platform/arch.
 * Returns the local path if cached, or downloads it first.
 *
 * The runner version is pinned to the Volt CLI version to ensure compatibility.
 */
export async function resolvePrebuiltRunner(options: {
  platform: string;
  arch: string;
  cacheDir: string;
}): Promise<string | null> {
  const version = getVoltCliVersion();
  if (version === '0.0.0') return null;

  const target = resolveRustTarget(options.platform, options.arch);
  if (!target) return null;

  const ext = options.platform === 'win32' ? '.exe' : '';
  const fileName = `volt-runner-${version}-${target}${ext}`;
  const cachedPath = resolve(options.cacheDir, fileName);

  if (existsSync(cachedPath)) {
    return cachedPath;
  }

  const downloadUrl = buildDownloadUrl(version, target, ext);
  if (!downloadUrl) return null;

  try {
    mkdirSync(options.cacheDir, { recursive: true });
    console.log(`[volt] Downloading pre-built runner from ${downloadUrl}...`);
    await downloadFile(downloadUrl, cachedPath);

    // Verify integrity via checksum manifest
    const checksumUrl = buildChecksumUrl(version);
    if (checksumUrl) {
      const verified = await verifyChecksum(cachedPath, fileName, checksumUrl);
      if (!verified) {
        // Delete the corrupted download
        try {
          const { unlinkSync } = await import('node:fs');
          unlinkSync(cachedPath);
        } catch { /* ignore cleanup error */ }
        console.warn('[volt] Checksum verification failed for pre-built runner. Falling back to Cargo.');
        return null;
      }
    }

    // Make executable on non-Windows platforms
    if (options.platform !== 'win32') {
      chmodSync(cachedPath, 0o755);
    }

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

function buildDownloadUrl(version: string, target: string, ext: string): string {
  return `https://github.com/voltkithq/Volt/releases/download/v${version}/volt-runner-${target}${ext}`;
}

function buildChecksumUrl(version: string): string {
  return `https://github.com/voltkithq/Volt/releases/download/v${version}/volt-runner-checksums.sha256`;
}

/**
 * Verify the SHA-256 checksum of a downloaded file against the release manifest.
 * The manifest format is one line per file: `<hex-hash>  <filename>`
 */
async function verifyChecksum(
  filePath: string,
  fileName: string,
  checksumUrl: string,
): Promise<boolean> {
  try {
    const manifest = await downloadToString(checksumUrl);
    const expectedHash = parseChecksumManifest(manifest, fileName);
    if (!expectedHash) {
      // No checksum entry for this file — skip verification
      return true;
    }

    const fileHash = computeSha256(filePath);
    return fileHash === expectedHash;
  } catch {
    // Can't fetch checksums — skip verification rather than blocking the build
    return true;
  }
}

function parseChecksumManifest(manifest: string, fileName: string): string | null {
  for (const line of manifest.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    // Format: "<hash>  <filename>" (two spaces between hash and name)
    const parts = trimmed.split(/\s+/);
    if (parts.length >= 2 && parts[1] === fileName) {
      return parts[0].toLowerCase();
    }
  }
  return null;
}

function computeSha256(filePath: string): string {
  const data = readFileSync(filePath);
  return createHash('sha256').update(data).digest('hex');
}

async function downloadToString(url: string): Promise<string> {
  const maxRedirects = 5;
  let currentUrl = url;

  for (let i = 0; i < maxRedirects; i++) {
    const result = await new Promise<string | { redirect: string }>((resolveReq, reject) => {
      const getter = currentUrl.startsWith('https') ? httpsGet : httpGet;
      const req = getter(currentUrl, (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          resolveReq({ redirect: res.headers.location });
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`HTTP ${res.statusCode}`));
          return;
        }
        const chunks: Buffer[] = [];
        res.on('data', (chunk: Buffer) => chunks.push(chunk));
        res.on('end', () => resolveReq(Buffer.concat(chunks).toString('utf8')));
        res.on('error', reject);
      });
      req.on('error', reject);
    });

    if (typeof result === 'string') return result;
    currentUrl = result.redirect;
  }

  throw new Error('Too many redirects');
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

    if (finalUrl === null) return;
    currentUrl = finalUrl;
  }

  throw new Error('Too many redirects');
}
