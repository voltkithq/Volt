import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

// The updater module imports getApp, so we need the app to be initialized
import { createApp, resetApp } from '../app.js';
import { autoUpdater, resetAutoUpdater } from '../updater.js';
import {
  updaterCheck,
  updaterDownloadAndVerify,
  updaterApply,
} from '@voltkit/volt-native';

describe('autoUpdater', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetApp();
    resetAutoUpdater();
    createApp({
      name: 'Test App',
      version: '1.0.0',
      permissions: ['fs', 'http'],
      updater: {
        endpoint: 'https://updates.example.com',
        publicKey: 'test-public-key-base64',
      },
    });
  });

  describe('checkForUpdates', () => {
    it('emits checking-for-update event', async () => {
      let emitted = false;
      autoUpdater.on('checking-for-update', () => { emitted = true; });
      await autoUpdater.checkForUpdates();
      expect(emitted).toBe(true);
    });

    it('emits update-not-available when no update found', async () => {
      vi.mocked(updaterCheck).mockResolvedValueOnce(null);
      let emitted = false;
      autoUpdater.on('update-not-available', () => { emitted = true; });
      const result = await autoUpdater.checkForUpdates();
      expect(result).toBeNull();
      expect(emitted).toBe(true);
    });

    it('emits update-available with info when update found', async () => {
      vi.mocked(updaterCheck).mockResolvedValueOnce({
        version: '2.0.0',
        url: 'https://updates.example.com/v2.0.0',
        signature: 'sig-base64',
        sha256: 'abc123',
      });
      let receivedInfo: unknown = null;
      autoUpdater.on('update-available', (info: unknown) => {
        receivedInfo = info;
      });
      const result = await autoUpdater.checkForUpdates();
      expect(result).not.toBeNull();
      expect(result!.version).toBe('2.0.0');
      expect(result!.url).toBe('https://updates.example.com/v2.0.0');
      expect(receivedInfo).toEqual(result);
    });

    it('emits error event and rejects when native throws', async () => {
      vi.mocked(updaterCheck).mockRejectedValueOnce(new Error('Network error'));
      let errorMsg = '';
      autoUpdater.on('error', (err: Error) => { errorMsg = err.message; });
      await expect(autoUpdater.checkForUpdates()).rejects.toThrow('Network error');
      expect(errorMsg).toBe('Network error');
    });

    it('rejects when required updater permissions are missing', async () => {
      resetApp();
      createApp({
        name: 'Test App',
        version: '1.0.0',
        permissions: ['fs'],
        updater: {
          endpoint: 'https://updates.example.com',
          publicKey: 'test-public-key-base64',
        },
      });

      await expect(autoUpdater.checkForUpdates()).rejects.toThrow(
        "requires 'http' in volt.config.ts permissions",
      );
      expect(updaterCheck).not.toHaveBeenCalled();
    });
  });

  describe('downloadUpdate', () => {
    it('emits update-downloaded on success', async () => {
      const info = {
        version: '2.0.0',
        url: 'https://updates.example.com/v2.0.0',
        signature: 'sig-base64',
        sha256: 'abc123',
      };
      let downloaded = false;
      autoUpdater.on('update-downloaded', () => { downloaded = true; });
      await autoUpdater.downloadUpdate(info);
      expect(downloaded).toBe(true);
      expect(updaterDownloadAndVerify).toHaveBeenCalled();
    });

    it('emits error and rethrows when download fails', async () => {
      vi.mocked(updaterDownloadAndVerify).mockRejectedValueOnce(
        new Error('Signature verification failed'),
      );
      const info = {
        version: '2.0.0',
        url: 'https://x.com/v2',
        signature: 'bad',
        sha256: '000',
      };
      let errorMsg = '';
      autoUpdater.on('error', (err: Error) => { errorMsg = err.message; });
      await expect(autoUpdater.downloadUpdate(info)).rejects.toThrow(
        'Signature verification failed',
      );
      expect(errorMsg).toBe('Signature verification failed');
    });
  });

  describe('quitAndInstall', () => {
    it('throws if no update has been downloaded', () => {
      expect(() => autoUpdater.quitAndInstall()).toThrow(
        'No update has been downloaded',
      );
    });

    it('calls updaterApply with downloaded data', async () => {
      const info = {
        version: '2.0.0',
        url: 'https://updates.example.com/v2.0.0',
        signature: 'sig-base64',
        sha256: 'abc123',
      };
      await autoUpdater.downloadUpdate(info);

      let quitEmitted = false;
      // `createApp` already called in beforeEach, so get singleton via dynamic import.
      const { getApp } = await import('../app.js');
      getApp().on('quit', () => {
        quitEmitted = true;
      });

      autoUpdater.quitAndInstall();
      expect(updaterApply).toHaveBeenCalled();
      expect(quitEmitted).toBe(true);
    });
  });
});
