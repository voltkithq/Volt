import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../../__mocks__/volt-native.js');
});

import { fs, setBaseDir } from '../../fs.js';
import { fsReadFileText } from '@voltkit/volt-native';

describe('fs path validation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setBaseDir('/mock/base');
  });

  it('rejects absolute paths starting with /', async () => {
    await expect(fs.readFile('/etc/passwd')).rejects.toThrow('Absolute paths are not allowed');
  });

  it('rejects absolute paths starting with backslash', async () => {
    await expect(fs.readFile('\\Windows\\System32\\cmd.exe')).rejects.toThrow(
      'Absolute paths are not allowed',
    );
  });

  it('rejects Windows drive letter paths', async () => {
    await expect(fs.readFile('C:\\secret.txt')).rejects.toThrow('Absolute paths are not allowed');
  });

  it('rejects path traversal with ..', async () => {
    await expect(fs.readFile('../../../etc/passwd')).rejects.toThrow(
      'Path traversal is not allowed',
    );
  });

  it('rejects embedded traversal', async () => {
    await expect(fs.readFile('data/../../../secret')).rejects.toThrow('Path traversal');
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
