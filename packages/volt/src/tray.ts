/**
 * System tray module.
 * Provides the `Tray` class for creating and managing system tray icons.
 * Requires `permissions: ['tray']` in volt.config.ts.
 */

import { EventEmitter } from 'node:events';
import { VoltTray } from '@voltkit/volt-native';
import { getApp } from './app.js';
import type { Permission } from './types.js';

/** Options for creating a system tray icon. */
export interface TrayOptions {
  /** Tooltip text shown on hover. */
  tooltip?: string;
  /** Path to the tray icon image (PNG). */
  icon?: string;
  /** Menu items to show in the tray context menu (currently unsupported). */
  menu?: TrayMenuItem[];
}

/** A tray context menu item. */
export interface TrayMenuItem {
  /** Display label. */
  label: string;
  /** Whether the item is enabled. Default: true. */
  enabled?: boolean;
  /** Item type: 'normal' or 'separator'. */
  type?: 'normal' | 'separator';
  /** Click handler. */
  click?: () => void;
}

/**
 * System tray icon.
 * Electron-compatible API for creating and managing system tray icons.
 *
 * @example
 * ```ts
 * const tray = new Tray({ tooltip: 'My App', icon: './icon.png' });
 * tray.on('click', () => { mainWindow.show(); });
 * ```
 */
export class Tray extends EventEmitter {
  private _tooltip: string;
  private _icon: string | undefined;
  private _visible: boolean = true;
  private _destroyed: boolean = false;
  private _native: VoltTray;

  constructor(options: TrayOptions = {}) {
    super();
    requireTrayPermissions(
      options.icon ? ['tray', 'fs'] : ['tray'],
      'new Tray()',
    );
    if (options.menu && options.menu.length > 0) {
      console.warn('[volt] Tray menu is not supported yet and will be ignored.');
    }
    this._tooltip = options.tooltip ?? '';
    this._icon = options.icon;

    // Create the native tray via napi binding
    this._native = new VoltTray({
      tooltip: this._tooltip,
      icon: this._icon,
    });

    // Wire native click events to the EventEmitter
    this._native.onClick((err: Error | null, eventJson: string) => {
      if (err || typeof eventJson !== 'string') {
        this.emit('click');
        return;
      }

      let event: unknown;
      try {
        event = JSON.parse(eventJson);
      } catch {
        event = undefined;
      }

      if (event !== undefined) {
        this.emit('click', event);
      } else {
        this.emit('click');
      }
    });
  }

  /** Set the tray tooltip text. */
  setToolTip(tooltip: string): void {
    requireTrayPermissions(['tray'], 'Tray.setToolTip()');
    this._tooltip = tooltip;
    if (!this._destroyed) {
      this._native.setTooltip(tooltip);
    }
  }

  /** Get the current tooltip text. */
  getToolTip(): string {
    return this._tooltip;
  }

  /** Set the tray icon from a file path. */
  setImage(iconPath: string): void {
    requireTrayPermissions(['tray', 'fs'], 'Tray.setImage()');
    this._icon = iconPath;
    if (!this._destroyed) {
      this._native.setIcon(iconPath);
    }
  }

  /** Show or hide the tray icon. */
  setVisible(visible: boolean): void {
    requireTrayPermissions(['tray'], 'Tray.setVisible()');
    this._visible = visible;
    if (!this._destroyed) {
      this._native.setVisible(visible);
    }
  }

  /** Check if the tray icon is visible. */
  isVisible(): boolean {
    return this._visible;
  }

  /** Destroy the tray icon and clean up resources. */
  destroy(): void {
    requireTrayPermissions(['tray'], 'Tray.destroy()');
    if (this._destroyed) return;
    this._destroyed = true;
    this._visible = false;
    this._native.destroy();
    this.removeAllListeners();
  }

  /** Check if the tray has been destroyed. */
  isDestroyed(): boolean {
    return this._destroyed;
  }
}

function requireTrayPermissions(required: readonly Permission[], apiName: string): void {
  const granted = new Set(getApp().getConfig().permissions ?? []);
  for (const permission of required) {
    if (!granted.has(permission)) {
      throw new Error(
        `Permission denied: ${apiName} requires '${permission}' in volt.config.ts permissions.`,
      );
    }
  }
}
