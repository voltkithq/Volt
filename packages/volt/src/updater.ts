/**
 * Auto-updater module.
 * Checks for updates, downloads with Ed25519 signature verification, and applies.
 *
 * @example
 * ```ts
 * import { autoUpdater } from 'voltkit';
 *
 * autoUpdater.on('update-available', (info) => {
 *   console.log(`Update available: ${info.version}`);
 * });
 *
 * const info = await autoUpdater.checkForUpdates();
 * if (info) {
 *   await autoUpdater.downloadUpdate(info);
 *   autoUpdater.quitAndInstall();
 * }
 * ```
 */

import { EventEmitter } from 'node:events';
import {
  type NativeUpdateConfig,
  type NativeUpdateInfo,
  updaterCheck,
  updaterDownloadAndVerify,
  updaterApply,
} from '@voltkit/volt-native';
import { getApp } from './app.js';
import type { Permission } from './types.js';

const REQUIRED_UPDATER_PERMISSIONS: readonly Permission[] = ['fs', 'http'];

function requireUpdaterPermissions(apiName: string): void {
  const granted = new Set(getApp().getConfig().permissions ?? []);
  for (const permission of REQUIRED_UPDATER_PERMISSIONS) {
    if (!granted.has(permission)) {
      throw new Error(
        `Permission denied: ${apiName} requires '${permission}' in volt.config.ts permissions.`,
      );
    }
  }
}

/** Information about an available update. */
export interface UpdateInfo {
  /** New version string (semver). */
  version: string;
  /** Download URL for the update binary. */
  url: string;
  /** Ed25519 signature (base64). */
  signature: string;
  /** SHA-256 hash (hex). */
  sha256: string;
  /** Target platform/architecture (e.g. `x86_64-unknown-linux-gnu`). */
  target: string;
}

/** Build the native UpdateConfig from the app configuration. */
function buildNativeConfig(apiName: string): NativeUpdateConfig {
  requireUpdaterPermissions(apiName);
  const app = getApp();
  const config = app.getConfig();
  const updater = config.updater;
  if (!updater) {
    throw new Error(
      'Updater not configured. Add an "updater" section to volt.config.ts with endpoint and publicKey.',
    );
  }
  return {
    endpoint: updater.endpoint,
    public_key: updater.publicKey,
    current_version: config.version ?? '0.0.0',
  };
}

class AutoUpdater extends EventEmitter {
  private _downloadedData: Buffer | null = null;

  /**
   * Check for available updates.
   * Emits 'checking-for-update', then 'update-available' or 'update-not-available'.
   */
  async checkForUpdates(): Promise<UpdateInfo | null> {
    this.emit('checking-for-update');

    try {
      const nativeConfig = buildNativeConfig('autoUpdater.checkForUpdates()');
      const result = await updaterCheck(nativeConfig);

      if (result) {
        const info: UpdateInfo = {
          version: result.version,
          url: result.url,
          signature: result.signature,
          sha256: result.sha256,
          target: result.target,
        };
        this.emit('update-available', info);
        return info;
      }

      this.emit('update-not-available');
      return null;
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      this.emit('error', error);
      throw error;
    }
  }

  /**
   * Download an update and verify its signature.
   * Emits 'download-progress' during download, then 'update-downloaded' on success.
   */
  async downloadUpdate(info: UpdateInfo): Promise<void> {
    try {
      const nativeConfig = buildNativeConfig('autoUpdater.downloadUpdate()');
      const nativeInfo: NativeUpdateInfo = {
        version: info.version,
        url: info.url,
        signature: info.signature,
        sha256: info.sha256,
        target: info.target,
      };

      const data = await updaterDownloadAndVerify(nativeConfig, nativeInfo);
      this._downloadedData = data;

      this.emit('update-downloaded', info);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      this.emit('error', new Error(message));
      throw err;
    }
  }

  /**
   * Quit the application and install the downloaded update.
   * On Windows production builds this uses the external updater helper process.
   */
  quitAndInstall(): void {
    requireUpdaterPermissions('autoUpdater.quitAndInstall()');
    if (!this._downloadedData) {
      throw new Error('No update has been downloaded. Call downloadUpdate() first.');
    }

    updaterApply(this._downloadedData);
    getApp().quit();
  }

  /** @internal Test-only reset hook. */
  _reset(): void {
    this._downloadedData = null;
    this.removeAllListeners();
  }
}

/** The auto-updater singleton instance. */
export const autoUpdater = new AutoUpdater();

/**
 * Reset the auto-updater singleton state.
 * Clears downloaded data and removes all listeners.
 * Intended for testing only.
 */
export function resetAutoUpdater(): void {
  autoUpdater._reset();
}
