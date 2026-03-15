import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../../__mocks__/volt-native.js');
});

import { fs, setBaseDir } from '../../fs.js';
import {
  fsCopy,
  fsExists,
  fsMkdir,
  fsReadDir,
  fsReadFile,
  fsReadFileText,
  fsRemove,
  fsRename,
  fsResolveGrant,
  fsStat,
  fsWriteFile,
} from '@voltkit/volt-native';

describe('fs operations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  it('calls native fsReadFileText with baseDir and path', async () => {
    const result = await fs.readFile('test.txt');
    expect(fsReadFileText).toHaveBeenCalledWith('/mock/base', 'test.txt');
    expect(result).toBe('mock file content');
  });

  it('calls native fsReadFile and returns Uint8Array', async () => {
    const result = await fs.readFileBinary('image.png');
    expect(fsReadFile).toHaveBeenCalledWith('/mock/base', 'image.png');
    expect(result).toBeInstanceOf(Uint8Array);
  });

  it('calls native fsWriteFile with buffer', async () => {
    await fs.writeFile('output.txt', 'hello world');
    expect(fsWriteFile).toHaveBeenCalledWith('/mock/base', 'output.txt', expect.any(Buffer));
  });

  it('calls native fsWriteFile with binary data', async () => {
    await fs.writeFileBinary('data.bin', new Uint8Array([1, 2, 3]));
    expect(fsWriteFile).toHaveBeenCalledWith('/mock/base', 'data.bin', expect.any(Buffer));
  });

  it('returns directory listing', async () => {
    const result = await fs.readDir('subdir');
    expect(fsReadDir).toHaveBeenCalledWith('/mock/base', 'subdir');
    expect(result).toEqual(['file1.txt', 'file2.txt']);
  });

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

  it('calls native fsExists with baseDir and path', async () => {
    const result = await fs.exists('test.txt');
    expect(fsExists).toHaveBeenCalledWith('/mock/base', 'test.txt');
    expect(result).toBe(true);
  });

  it('rejects invalid exists paths', async () => {
    await expect(fs.exists('/etc/passwd')).rejects.toThrow('Absolute paths are not allowed');
    await expect(fs.exists('../../secret')).rejects.toThrow('Path traversal is not allowed');
  });

  it('calls native fsMkdir', async () => {
    await fs.mkdir('new-dir/sub');
    expect(fsMkdir).toHaveBeenCalledWith('/mock/base', 'new-dir/sub');
  });

  it('calls native fsRemove', async () => {
    await fs.remove('old-file.txt');
    expect(fsRemove).toHaveBeenCalledWith('/mock/base', 'old-file.txt');
  });
});

describe('scoped fs operations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  it('resolves grant and returns a scoped handle', async () => {
    const scopedFs = await fs.bindScope('test_grant_123');
    expect(fsResolveGrant).toHaveBeenCalledWith('test_grant_123');
    expect(typeof scopedFs.readFile).toBe('function');
    expect(typeof scopedFs.watch).toBe('function');
  });

  it('supports scoped reads and metadata', async () => {
    const scopedFs = await fs.bindScope('test_grant_scope');
    await scopedFs.readFile('notes/readme.md');
    await scopedFs.readDir('notes');
    const info = await scopedFs.stat('test.md');
    const exists = await scopedFs.exists('test.md');

    expect(fsReadFileText).toHaveBeenCalledWith('/mock/grant/path', 'notes/readme.md');
    expect(fsReadDir).toHaveBeenCalledWith('/mock/grant/path', 'notes');
    expect(fsStat).toHaveBeenCalledWith('/mock/grant/path', 'test.md');
    expect(info.modifiedMs).toBe(1700000000000);
    expect(exists).toBe(true);
  });

  it('allows the scope root for readDir', async () => {
    const scopedFs = await fs.bindScope('test_grant_root');
    await scopedFs.readDir('');
    expect(fsReadDir).toHaveBeenCalledWith('/mock/grant/path', '');
  });

  it('rejects invalid grant IDs', async () => {
    await expect(fs.bindScope('')).rejects.toThrow('FS_SCOPE_INVALID');

    vi.mocked(fsResolveGrant).mockImplementationOnce(() => {
      throw new Error('FS_SCOPE_INVALID: grant ID not found or expired');
    });

    await expect(fs.bindScope('bad_grant')).rejects.toThrow('FS_SCOPE_INVALID');
  });

  it('rejects invalid scoped read paths', async () => {
    const scopedFs = await fs.bindScope('test_grant_paths');
    await expect(scopedFs.readFile('../../etc/passwd')).rejects.toThrow('Path traversal');
    await expect(scopedFs.readFile('/etc/passwd')).rejects.toThrow('Absolute paths');
  });

  it('supports scoped writes and copy operations', async () => {
    const scopedFs = await fs.bindScope('test_grant_writes');

    await scopedFs.writeFile('notes/new.md', '# Hello');
    await scopedFs.writeFileBinary('data.bin', new Uint8Array([1, 2, 3]));
    await scopedFs.mkdir('new-dir/sub');
    await scopedFs.remove('old-file.txt');
    await scopedFs.rename('old.md', 'new.md');
    await scopedFs.copy('original.md', 'duplicate.md');

    expect(fsWriteFile).toHaveBeenCalledWith(
      '/mock/grant/path',
      'notes/new.md',
      expect.any(Buffer),
    );
    expect(fsWriteFile).toHaveBeenCalledWith('/mock/grant/path', 'data.bin', expect.any(Buffer));
    expect(fsMkdir).toHaveBeenCalledWith('/mock/grant/path', 'new-dir/sub');
    expect(fsRemove).toHaveBeenCalledWith('/mock/grant/path', 'old-file.txt');
    expect(fsRename).toHaveBeenCalledWith('/mock/grant/path', 'old.md', 'new.md');
    expect(fsCopy).toHaveBeenCalledWith('/mock/grant/path', 'original.md', 'duplicate.md');
  });

  it('rejects invalid scoped write paths', async () => {
    const scopedFs = await fs.bindScope('test_grant_write_checks');

    await expect(scopedFs.writeFile('../../etc/evil', 'bad')).rejects.toThrow('Path traversal');
    await expect(scopedFs.rename('file.md', '../../etc/evil')).rejects.toThrow('Path traversal');
    await expect(scopedFs.rename('../../etc/passwd', 'stolen.md')).rejects.toThrow(
      'Path traversal',
    );
    await expect(scopedFs.copy('/etc/passwd', 'stolen.md')).rejects.toThrow('Absolute paths');
    await expect(scopedFs.copy('file.md', '/tmp/evil')).rejects.toThrow('Absolute paths');
  });
});
