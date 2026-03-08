import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the native module before importing fs
vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { fs, setBaseDir } from '../fs.js';
import {
  fsReadFileText,
  fsReadFile,
  fsWriteFile,
  fsReadDir,
  fsStat,
  fsMkdir,
  fsRemove,
} from '@voltkit/volt-native';

describe('fs module', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  describe('path validation', () => {
    it('rejects absolute paths starting with /', async () => {
      await expect(fs.readFile('/etc/passwd')).rejects.toThrow(
        'Absolute paths are not allowed',
      );
    });

    it('rejects absolute paths starting with backslash', async () => {
      await expect(fs.readFile('\\Windows\\System32\\cmd.exe')).rejects.toThrow(
        'Absolute paths are not allowed',
      );
    });

    it('rejects Windows drive letter paths', async () => {
      await expect(fs.readFile('C:\\secret.txt')).rejects.toThrow(
        'Absolute paths are not allowed',
      );
    });

    it('rejects path traversal with ..', async () => {
      await expect(fs.readFile('../../../etc/passwd')).rejects.toThrow(
        'Path traversal is not allowed',
      );
    });

    it('rejects embedded traversal', async () => {
      await expect(fs.readFile('data/../../../secret')).rejects.toThrow(
        'Path traversal',
      );
    });

    it('rejects null-byte paths', async () => {
      await expect(fs.readFile('data/config.json\0evil')).rejects.toThrow(
        'Null bytes are not allowed',
      );
    });

    it('allows names that contain dots but not traversal segments', async () => {
      await fs.readFile('config..backup.json');
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', 'config..backup.json');
    });

    it('allows simple relative paths', async () => {
      await fs.readFile('data/config.json');
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', 'data/config.json');
    });

    it('allows single-segment paths', async () => {
      await fs.readFile('readme.txt');
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', 'readme.txt');
    });

    it('rejects empty relative path', async () => {
      await expect(fs.readDir('')).rejects.toThrow('Path cannot be empty');
    });

    it('handles very long relative paths', async () => {
      const longPath = `data/${'nested/'.repeat(40)}file.txt`;
      await fs.readFile(longPath);
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', longPath);
    });
  });

  describe('readFile', () => {
    it('calls native fsReadFileText with baseDir and path', async () => {
      const result = await fs.readFile('test.txt');
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', 'test.txt');
      expect(result).toBe('mock file content');
    });
  });

  describe('readFileBinary', () => {
    it('calls native fsReadFile and returns Uint8Array', async () => {
      const result = await fs.readFileBinary('image.png');
      expect(fsReadFile).toHaveBeenCalledWith('/mock/base', 'image.png');
      expect(result).toBeInstanceOf(Uint8Array);
    });
  });

  describe('writeFile', () => {
    it('calls native fsWriteFile with buffer', async () => {
      await fs.writeFile('output.txt', 'hello world');
      expect(fsWriteFile).toHaveBeenCalledWith(
        '/mock/base',
        'output.txt',
        expect.any(Buffer),
      );
    });
  });

  describe('writeFileBinary', () => {
    it('calls native fsWriteFile with binary data', async () => {
      const data = new Uint8Array([1, 2, 3]);
      await fs.writeFileBinary('data.bin', data);
      expect(fsWriteFile).toHaveBeenCalledWith(
        '/mock/base',
        'data.bin',
        expect.any(Buffer),
      );
    });
  });

  describe('readDir', () => {
    it('returns directory listing', async () => {
      const result = await fs.readDir('subdir');
      expect(fsReadDir).toHaveBeenCalledWith('/mock/base', 'subdir');
      expect(result).toEqual(['file1.txt', 'file2.txt']);
    });
  });

  describe('stat', () => {
    it('returns file metadata', async () => {
      const info = await fs.stat('test.txt');
      expect(fsStat).toHaveBeenCalledWith('/mock/base', 'test.txt');
      expect(info).toEqual({
        size: 1024,
        isFile: true,
        isDir: false,
        readonly: false,
      });
    });
  });

  describe('mkdir', () => {
    it('calls native fsMkdir', async () => {
      await fs.mkdir('new-dir/sub');
      expect(fsMkdir).toHaveBeenCalledWith('/mock/base', 'new-dir/sub');
    });
  });

  describe('remove', () => {
    it('calls native fsRemove', async () => {
      await fs.remove('old-file.txt');
      expect(fsRemove).toHaveBeenCalledWith('/mock/base', 'old-file.txt');
    });
  });
});
