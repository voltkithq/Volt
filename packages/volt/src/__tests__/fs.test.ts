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
  fsExists,
  fsMkdir,
  fsRemove,
  fsResolveGrant,
  fsRename,
  fsCopy,
  fsWatchStart,
  fsWatchPoll,
  fsWatchClose,
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
    it('returns file metadata with timestamps', async () => {
      const info = await fs.stat('test.txt');
      expect(fsStat).toHaveBeenCalledWith('/mock/base', 'test.txt');
      expect(info).toEqual({
        size: 1024,
        isFile: true,
        isDir: false,
        readonly: false,
        modifiedMs: 1700000000000,
        createdMs: 1699000000000,
      });
    });
  });

  describe('exists', () => {
    it('calls native fsExists with baseDir and path', async () => {
      const result = await fs.exists('test.txt');
      expect(fsExists).toHaveBeenCalledWith('/mock/base', 'test.txt');
      expect(result).toBe(true);
    });

    it('rejects absolute paths', async () => {
      await expect(fs.exists('/etc/passwd')).rejects.toThrow(
        'Absolute paths are not allowed',
      );
    });

    it('rejects path traversal', async () => {
      await expect(fs.exists('../../secret')).rejects.toThrow(
        'Path traversal is not allowed',
      );
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

  describe('bindScope', () => {
    it('resolves grant and returns scoped handle', async () => {
      const scopedFs = await fs.bindScope('test_grant_123');
      expect(fsResolveGrant).toHaveBeenCalledWith('test_grant_123');
      expect(scopedFs).toBeDefined();
      expect(typeof scopedFs.readFile).toBe('function');
      expect(typeof scopedFs.readFileBinary).toBe('function');
      expect(typeof scopedFs.readDir).toBe('function');
      expect(typeof scopedFs.stat).toBe('function');
      expect(typeof scopedFs.exists).toBe('function');
    });

    it('scoped readFile calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_456');
      await scopedFs.readFile('notes/readme.md');
      expect(fsReadFileText).toHaveBeenCalledWith('/mock/grant/path', 'notes/readme.md');
    });

    it('scoped readDir calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_789');
      await scopedFs.readDir('notes');
      expect(fsReadDir).toHaveBeenCalledWith('/mock/grant/path', 'notes');
    });

    it('scoped stat calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_stat');
      const info = await scopedFs.stat('test.md');
      expect(fsStat).toHaveBeenCalledWith('/mock/grant/path', 'test.md');
      expect(info.modifiedMs).toBe(1700000000000);
    });

    it('scoped exists calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_exists');
      const result = await scopedFs.exists('test.md');
      expect(fsExists).toHaveBeenCalledWith('/mock/grant/path', 'test.md');
      expect(result).toBe(true);
    });

    it('scoped readDir allows empty string for scope root', async () => {
      const scopedFs = await fs.bindScope('test_grant_root');
      await scopedFs.readDir('');
      expect(fsReadDir).toHaveBeenCalledWith('/mock/grant/path', '');
    });

    it('rejects empty grant ID', async () => {
      await expect(fs.bindScope('')).rejects.toThrow('FS_SCOPE_INVALID');
    });

    it('rejects invalid grant ID from native', async () => {
      vi.mocked(fsResolveGrant).mockImplementationOnce(() => {
        throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
      });
      await expect(fs.bindScope('bad_grant')).rejects.toThrow('FS_SCOPE_INVALID');
    });

    it('scoped readFile rejects path traversal', async () => {
      const scopedFs = await fs.bindScope('test_grant_traversal');
      await expect(scopedFs.readFile('../../etc/passwd')).rejects.toThrow('Path traversal');
    });

    it('scoped readFile rejects absolute paths', async () => {
      const scopedFs = await fs.bindScope('test_grant_abs');
      await expect(scopedFs.readFile('/etc/passwd')).rejects.toThrow('Absolute paths');
    });
  });

  describe('scoped write operations', () => {
    it('scoped writeFile calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_write');
      await scopedFs.writeFile('notes/new.md', '# Hello');
      expect(fsWriteFile).toHaveBeenCalledWith(
        '/mock/grant/path',
        'notes/new.md',
        expect.any(Buffer),
      );
    });

    it('scoped writeFileBinary calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_write_bin');
      const data = new Uint8Array([1, 2, 3]);
      await scopedFs.writeFileBinary('data.bin', data);
      expect(fsWriteFile).toHaveBeenCalledWith(
        '/mock/grant/path',
        'data.bin',
        expect.any(Buffer),
      );
    });

    it('scoped mkdir calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_mkdir');
      await scopedFs.mkdir('new-dir/sub');
      expect(fsMkdir).toHaveBeenCalledWith('/mock/grant/path', 'new-dir/sub');
    });

    it('scoped remove calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_remove');
      await scopedFs.remove('old-file.txt');
      expect(fsRemove).toHaveBeenCalledWith('/mock/grant/path', 'old-file.txt');
    });

    it('scoped rename calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_rename');
      await scopedFs.rename('old.md', 'new.md');
      expect(fsRename).toHaveBeenCalledWith('/mock/grant/path', 'old.md', 'new.md');
    });

    it('scoped copy calls native with grant path', async () => {
      const scopedFs = await fs.bindScope('test_grant_copy');
      await scopedFs.copy('original.md', 'duplicate.md');
      expect(fsCopy).toHaveBeenCalledWith('/mock/grant/path', 'original.md', 'duplicate.md');
    });

    it('scoped write rejects path traversal', async () => {
      const scopedFs = await fs.bindScope('test_grant_write_trav');
      await expect(scopedFs.writeFile('../../etc/evil', 'bad')).rejects.toThrow('Path traversal');
    });

    it('scoped rename rejects path traversal', async () => {
      const scopedFs = await fs.bindScope('test_grant_rename_trav');
      await expect(scopedFs.rename('file.md', '../../etc/evil')).rejects.toThrow('Path traversal');
      await expect(scopedFs.rename('../../etc/passwd', 'stolen.md')).rejects.toThrow(
        'Path traversal',
      );
    });

    it('scoped copy rejects absolute paths', async () => {
      const scopedFs = await fs.bindScope('test_grant_copy_abs');
      await expect(scopedFs.copy('/etc/passwd', 'stolen.md')).rejects.toThrow('Absolute paths');
      await expect(scopedFs.copy('file.md', '/tmp/evil')).rejects.toThrow('Absolute paths');
    });
  });

  describe('watch', () => {
    it('creates a watcher and returns a FileWatcher handle', async () => {
      const watcher = await fs.watch('data');
      expect(fsWatchStart).toHaveBeenCalledWith('/mock/base', 'data', true, 200);
      expect(watcher).toBeDefined();
      expect(typeof watcher.poll).toBe('function');
      expect(typeof watcher.close).toBe('function');
    });

    it('passes custom watch options', async () => {
      await fs.watch('logs', { recursive: false, debounceMs: 500 });
      expect(fsWatchStart).toHaveBeenCalledWith('/mock/base', 'logs', false, 500);
    });

    it('poll calls native fsWatchPoll with watcher ID', async () => {
      const watcher = await fs.watch('data');
      const events = await watcher.poll();
      expect(fsWatchPoll).toHaveBeenCalled();
      expect(events).toEqual([]);
    });

    it('close calls native fsWatchClose with watcher ID', async () => {
      const watcher = await fs.watch('data');
      await watcher.close();
      expect(fsWatchClose).toHaveBeenCalled();
    });

    it('rejects watching absolute paths', async () => {
      await expect(fs.watch('/etc')).rejects.toThrow('Absolute paths');
    });

    it('rejects watching with path traversal', async () => {
      await expect(fs.watch('../../secret')).rejects.toThrow('Path traversal');
    });
  });

  describe('scoped watch', () => {
    it('creates a scoped watcher via ScopedFs.watch', async () => {
      const scopedFs = await fs.bindScope('test_grant_watch');
      const watcher = await scopedFs.watch('subdir');
      expect(fsWatchStart).toHaveBeenCalledWith('/mock/grant/path', 'subdir', true, 200);
      expect(watcher).toBeDefined();
    });

    it('scoped watch allows empty subpath for scope root', async () => {
      const scopedFs = await fs.bindScope('test_grant_watch_root');
      await scopedFs.watch('');
      expect(fsWatchStart).toHaveBeenCalledWith('/mock/grant/path', '', true, 200);
    });

    it('scoped watch rejects path traversal', async () => {
      const scopedFs = await fs.bindScope('test_grant_watch_trav');
      await expect(scopedFs.watch('../../secret')).rejects.toThrow('Path traversal');
    });

    it('scoped watch rejects absolute paths', async () => {
      const scopedFs = await fs.bindScope('test_grant_watch_abs');
      await expect(scopedFs.watch('/etc')).rejects.toThrow('Absolute paths');
    });
  });
});
