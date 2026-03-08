import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  validateBuildOutput,
  createAssetBundle,
} from '../utils/bundler.js';
import { mkdirSync, writeFileSync, rmSync, existsSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

describe('bundler utilities', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = join(tmpdir(), `volt-bundler-test-${Date.now()}`);
    mkdirSync(tempDir, { recursive: true });
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('validateBuildOutput', () => {
    it('returns false when directory does not exist', () => {
      expect(validateBuildOutput(tempDir, 'nonexistent')).toBe(false);
    });

    it('returns false when index.html is missing', () => {
      const outDir = join(tempDir, 'dist');
      mkdirSync(outDir);
      writeFileSync(join(outDir, 'bundle.js'), 'console.log("hi")');
      expect(validateBuildOutput(tempDir, 'dist')).toBe(false);
    });

    it('returns true when directory has index.html', () => {
      const outDir = join(tempDir, 'dist');
      mkdirSync(outDir);
      writeFileSync(join(outDir, 'index.html'), '<html></html>');
      expect(validateBuildOutput(tempDir, 'dist')).toBe(true);
    });
  });

  describe('createAssetBundle', () => {
    it('creates correct binary format for single file', () => {
      const distDir = join(tempDir, 'dist');
      mkdirSync(distDir);
      writeFileSync(join(distDir, 'index.html'), '<h1>Hello</h1>');

      const bundle = createAssetBundle(distDir);

      // First 4 bytes: file count (u32le)
      const fileCount = bundle.readUInt32LE(0);
      expect(fileCount).toBe(1);

      // Next: path_len(u32le) + path_bytes + data_len(u32le) + data_bytes
      let offset = 4;
      const pathLen = bundle.readUInt32LE(offset);
      offset += 4;
      const path = bundle.subarray(offset, offset + pathLen).toString('utf-8');
      offset += pathLen;
      expect(path).toBe('index.html');

      const dataLen = bundle.readUInt32LE(offset);
      offset += 4;
      const data = bundle.subarray(offset, offset + dataLen).toString('utf-8');
      expect(data).toBe('<h1>Hello</h1>');
    });

    it('includes files from subdirectories', () => {
      const distDir = join(tempDir, 'dist');
      mkdirSync(join(distDir, 'assets', 'css'), { recursive: true });
      writeFileSync(join(distDir, 'index.html'), '<html></html>');
      writeFileSync(join(distDir, 'assets', 'css', 'style.css'), 'body{}');

      const bundle = createAssetBundle(distDir);
      const fileCount = bundle.readUInt32LE(0);
      expect(fileCount).toBe(2);

      // Parse both entries to check paths use forward slashes
      let offset = 4;
      const paths: string[] = [];
      for (let i = 0; i < fileCount; i++) {
        const pathLen = bundle.readUInt32LE(offset);
        offset += 4;
        const p = bundle.subarray(offset, offset + pathLen).toString('utf-8');
        paths.push(p);
        offset += pathLen;
        const dataLen = bundle.readUInt32LE(offset);
        offset += 4 + dataLen;
      }
      expect(paths).toContain('index.html');
      expect(paths).toContain('assets/css/style.css');
    });

    it('creates empty bundle for empty directory', () => {
      const distDir = join(tempDir, 'empty');
      mkdirSync(distDir);

      const bundle = createAssetBundle(distDir);
      const fileCount = bundle.readUInt32LE(0);
      expect(fileCount).toBe(0);
      expect(bundle.length).toBe(4); // just the count header
    });

    it('handles binary files correctly', () => {
      const distDir = join(tempDir, 'dist');
      mkdirSync(distDir);
      const binaryData = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x00, 0xff]);
      writeFileSync(join(distDir, 'icon.png'), binaryData);

      const bundle = createAssetBundle(distDir);
      const fileCount = bundle.readUInt32LE(0);
      expect(fileCount).toBe(1);

      let offset = 4;
      const pathLen = bundle.readUInt32LE(offset);
      offset += 4 + pathLen;
      const dataLen = bundle.readUInt32LE(offset);
      offset += 4;
      const data = bundle.subarray(offset, offset + dataLen);
      expect(Buffer.compare(data, binaryData)).toBe(0);
    });

    it('rejects symlinked files to prevent out-of-tree inclusion', () => {
      const distDir = join(tempDir, 'dist');
      mkdirSync(distDir);

      const externalFile = join(tempDir, 'outside.txt');
      writeFileSync(externalFile, 'outside');

      const symlinkPath = join(distDir, 'linked.txt');
      try {
        symlinkSync(externalFile, symlinkPath);
      } catch (error) {
        const code = (error as NodeJS.ErrnoException).code;
        if (code === 'EPERM' || code === 'EACCES' || code === 'UNKNOWN') {
          // Some environments (especially Windows without developer mode) disallow symlink creation.
          return;
        }
        throw error;
      }

      expect(() => createAssetBundle(distDir)).toThrow(/Symbolic links are not allowed/);
    });
  });
});
