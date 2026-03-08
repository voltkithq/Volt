import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { shell } from '../shell.js';
import { shellOpenExternal } from '@voltkit/volt-native';

describe('shell module', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('openExternal', () => {
    it('allows https URLs', async () => {
      await shell.openExternal('https://example.com');
      expect(shellOpenExternal).toHaveBeenCalledWith('https://example.com');
    });

    it('allows http URLs', async () => {
      await shell.openExternal('http://example.com');
      expect(shellOpenExternal).toHaveBeenCalledWith('http://example.com');
    });

    it('allows mailto URLs', async () => {
      await shell.openExternal('mailto:user@example.com');
      expect(shellOpenExternal).toHaveBeenCalledWith(
        'mailto:user@example.com',
      );
    });

    it('rejects file:// protocol', async () => {
      await expect(
        shell.openExternal('file:///etc/passwd'),
      ).rejects.toThrow('Protocol not allowed');
    });

    it('rejects javascript: protocol', async () => {
      await expect(
        shell.openExternal('javascript:alert(1)'),
      ).rejects.toThrow('Protocol not allowed');
    });

    it('rejects data: protocol', async () => {
      await expect(
        shell.openExternal('data:text/html,<h1>XSS</h1>'),
      ).rejects.toThrow('Protocol not allowed');
    });

    it('rejects ftp: protocol', async () => {
      await expect(
        shell.openExternal('ftp://files.example.com/data.zip'),
      ).rejects.toThrow('Protocol not allowed');
    });

    it('rejects invalid URLs', async () => {
      await expect(shell.openExternal('not a url')).rejects.toThrow(
        'Invalid URL',
      );
    });

    it('does not call native on rejected URL', async () => {
      try {
        await shell.openExternal('file:///etc/shadow');
      } catch {
        // expected
      }
      expect(shellOpenExternal).not.toHaveBeenCalled();
    });
  });
});
