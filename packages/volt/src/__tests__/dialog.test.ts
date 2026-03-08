import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { dialog } from '../dialog.js';
import {
  dialogShowOpen,
  dialogShowSave,
  dialogShowMessage,
} from '@voltkit/volt-native';

describe('dialog module', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('showOpenDialog', () => {
    it('returns file paths from native', async () => {
      const result = await dialog.showOpenDialog({
        title: 'Select File',
        filters: [{ name: 'Text', extensions: ['txt'] }],
      });
      expect(result.canceled).toBe(false);
      expect(result.filePaths).toEqual(['/mock/path/file.txt']);
      expect(dialogShowOpen).toHaveBeenCalledWith({
        title: 'Select File',
        default_path: undefined,
        filters: [{ name: 'Text', extensions: ['txt'] }],
        multiple: false,
        directory: false,
      });
    });

    it('reports canceled when native returns empty array', async () => {
      vi.mocked(dialogShowOpen).mockReturnValueOnce([]);
      const result = await dialog.showOpenDialog();
      expect(result.canceled).toBe(true);
      expect(result.filePaths).toEqual([]);
    });

    it('maps multiSelections and directory options', async () => {
      await dialog.showOpenDialog({
        multiSelections: true,
        directory: true,
        defaultPath: '/home',
      });
      expect(dialogShowOpen).toHaveBeenCalledWith(
        expect.objectContaining({
          multiple: true,
          directory: true,
          default_path: '/home',
        }),
      );
    });
  });

  describe('showSaveDialog', () => {
    it('returns file path from native', async () => {
      const result = await dialog.showSaveDialog({
        title: 'Save File',
        defaultPath: '/home/doc.txt',
      });
      expect(result.canceled).toBe(false);
      expect(result.filePath).toBe('/mock/path/save.txt');
    });

    it('reports canceled when native returns null', async () => {
      vi.mocked(dialogShowSave).mockReturnValueOnce(null);
      const result = await dialog.showSaveDialog();
      expect(result.canceled).toBe(true);
      expect(result.filePath).toBe('');
    });
  });

  describe('showMessageBox', () => {
    it('returns confirmed true from native', async () => {
      const result = await dialog.showMessageBox({
        message: 'Are you sure?',
        type: 'warning',
        title: 'Confirm',
        buttons: ['Yes', 'No'],
      });
      expect(result.confirmed).toBe(true);
      expect(dialogShowMessage).toHaveBeenCalledWith({
        dialog_type: 'warning',
        title: 'Confirm',
        message: 'Are you sure?',
        buttons: ['Yes', 'No'],
      });
    });

    it('defaults to info type and empty buttons', async () => {
      await dialog.showMessageBox({ message: 'Hello' });
      expect(dialogShowMessage).toHaveBeenCalledWith({
        dialog_type: 'info',
        title: '',
        message: 'Hello',
        buttons: [],
      });
    });

    it('returns confirmed false when native returns false', async () => {
      vi.mocked(dialogShowMessage).mockReturnValueOnce(false);
      const result = await dialog.showMessageBox({ message: 'test' });
      expect(result.confirmed).toBe(false);
    });
  });
});
