/**
 * Frontend asset bundling utilities.
 * Reads Vite build output and creates a binary asset bundle
 * that the Rust binary can embed via include_bytes!.
 */

import { resolve, relative, join, isAbsolute } from 'node:path';
import {
  existsSync,
  readFileSync,
  writeFileSync,
  readdirSync,
  statSync,
  lstatSync,
  realpathSync,
} from 'node:fs';

/** Check if a Vite build output directory exists and has content. */
export function validateBuildOutput(projectRoot: string, outDir: string): boolean {
  const fullPath = resolve(projectRoot, outDir);
  if (!existsSync(fullPath)) {
    return false;
  }

  const indexHtml = resolve(fullPath, 'index.html');
  return existsSync(indexHtml);
}

/** Collect all files from a directory recursively, returning [relativePath, absolutePath] pairs. */
function collectFiles(dir: string, base: string): Array<[string, string]> {
  const results: Array<[string, string]> = [];

  for (const entry of readdirSync(dir)) {
    const fullPath = join(dir, entry);
    const lstat = lstatSync(fullPath);
    if (lstat.isSymbolicLink()) {
      throw new Error(`Symbolic links are not allowed in build output: ${fullPath}`);
    }

    const resolvedPath = realpathSync(fullPath);
    assertPathWithinBase(base, resolvedPath);
    const stat = statSync(resolvedPath);

    if (stat.isDirectory()) {
      results.push(...collectFiles(resolvedPath, base));
    } else if (stat.isFile()) {
      const relPath = relative(base, resolvedPath).replace(/\\/g, '/');
      results.push([relPath, resolvedPath]);
    }
  }

  return results;
}

function assertPathWithinBase(baseDir: string, candidatePath: string): void {
  const relPath = relative(baseDir, candidatePath);
  const isInsideBase =
    relPath === '' || (!relPath.startsWith('..') && !isAbsolute(relPath));

  if (!isInsideBase) {
    throw new Error(`Asset path escapes build output directory: ${candidatePath}`);
  }
}

/**
 * Create a binary asset bundle from a directory.
 * Format: count(u32le) + [path_len(u32le) + path_bytes + data_len(u32le) + data_bytes] * count
 * This matches the Rust AssetBundle::from_bytes() format in volt-core/src/embed.rs.
 */
export function createAssetBundle(distDir: string): Buffer {
  const distRoot = realpathSync(distDir);
  const files = collectFiles(distRoot, distRoot);
  const parts: Buffer[] = [];

  const MAX_U32 = 0xFFFF_FFFF;
  if (files.length > MAX_U32) {
    throw new Error(`Too many asset files (${files.length}); max supported is ${MAX_U32}.`);
  }

  // Write file count
  const countBuf = Buffer.alloc(4);
  countBuf.writeUInt32LE(files.length, 0);
  parts.push(countBuf);

  for (const [relPath, absPath] of files) {
    const pathBytes = Buffer.from(relPath, 'utf-8');
    const dataBytes = readFileSync(absPath);
    if (pathBytes.length > MAX_U32) {
      throw new Error(`Asset path is too long for bundle format: ${relPath}`);
    }
    if (dataBytes.length > MAX_U32) {
      throw new Error(`Asset exceeds 4GiB bundle entry limit: ${relPath}`);
    }

    // Path length + path bytes
    const pathLenBuf = Buffer.alloc(4);
    pathLenBuf.writeUInt32LE(pathBytes.length, 0);
    parts.push(pathLenBuf);
    parts.push(pathBytes);

    // Data length + data bytes
    const dataLenBuf = Buffer.alloc(4);
    dataLenBuf.writeUInt32LE(dataBytes.length, 0);
    parts.push(dataLenBuf);
    parts.push(dataBytes);
  }

  return Buffer.concat(parts);
}

/**
 * Write the asset bundle to a file that can be embedded via include_bytes!.
 * Returns the path to the bundle file.
 */
export function writeAssetBundle(projectRoot: string, distDir: string, outputName: string): string {
  const distPath = resolve(projectRoot, distDir);

  if (!validateBuildOutput(projectRoot, distDir)) {
    throw new Error(`Build output not found at ${distPath}. Run 'vite build' first.`);
  }

  const bundle = createAssetBundle(distPath);
  const outputPath = resolve(projectRoot, outputName);
  writeFileSync(outputPath, bundle);

  return outputPath;
}
