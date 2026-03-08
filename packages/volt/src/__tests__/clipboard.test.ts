import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { clipboard } from '../clipboard.js';
import {
  clipboardReadText,
  clipboardWriteText,
  clipboardReadImage,
  clipboardWriteImage,
} from '@voltkit/volt-native';

describe('clipboard module', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('readText', () => {
    it('calls native clipboardReadText', () => {
      const result = clipboard.readText();
      expect(clipboardReadText).toHaveBeenCalled();
      expect(typeof result).toBe('string');
    });
  });

  describe('writeText', () => {
    it('calls native clipboardWriteText with the text', () => {
      clipboard.writeText('hello clipboard');
      expect(clipboardWriteText).toHaveBeenCalledWith('hello clipboard');
    });
  });

  describe('readImage', () => {
    it('returns ClipboardImage with Uint8Array rgba', () => {
      const img = clipboard.readImage();
      expect(img).not.toBeNull();
      expect(img!.rgba).toBeInstanceOf(Uint8Array);
      expect(img!.width).toBe(2);
      expect(img!.height).toBe(2);
    });

    it('returns null when native throws', () => {
      vi.mocked(clipboardReadImage).mockImplementationOnce(() => {
        throw new Error('No image in clipboard');
      });
      const result = clipboard.readImage();
      expect(result).toBeNull();
    });
  });

  describe('writeImage', () => {
    it('calls native clipboardWriteImage with converted Buffer', () => {
      const image = {
        rgba: new Uint8Array([255, 0, 0, 255]),
        width: 1,
        height: 1,
      };
      clipboard.writeImage(image);
      expect(clipboardWriteImage).toHaveBeenCalledWith({
        rgba: expect.any(Buffer),
        width: 1,
        height: 1,
      });
    });
  });
});
