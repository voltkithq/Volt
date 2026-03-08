import { EventEmitter } from 'node:events';
import type { VoltConfig } from './types.js';

/** Events emitted by the VoltApp. */
export interface AppEvents {
  ready: [];
  'window-all-closed': [];
  'before-quit': [];
  quit: [];
}

/**
 * Main application class managing the lifecycle of a Volt desktop app.
 * Follows Electron's `app` event pattern for familiar DX.
 */
export class VoltApp extends EventEmitter {
  private config: VoltConfig;
  private isReady = false;
  private nativeApp: unknown = null;

  constructor(config: VoltConfig) {
    super();
    this.config = config;
  }

  /** Get the application name. */
  getName(): string {
    return this.config.name;
  }

  /** Get the application version. */
  getVersion(): string {
    return this.config.version ?? '0.0.0';
  }

  /** Check if the app has finished initializing. */
  get ready(): boolean {
    return this.isReady;
  }

  /** Get the full application config. */
  getConfig(): VoltConfig {
    return structuredClone(this.config);
  }

  /**
   * Get the native app instance. Used internally by BrowserWindow and other APIs.
   * @internal
   */
  getNativeApp(): unknown {
    return this.nativeApp;
  }

  /**
   * Set the native app instance. Called by the CLI when initializing the native layer.
   * @internal
   */
  setNativeApp(native: unknown): void {
    this.nativeApp = native;
  }

  /**
   * Mark the app as ready and emit the 'ready' event.
   * Called internally after native initialization completes.
   * @internal
   */
  markReady(): void {
    if (!this.isReady) {
      this.isReady = true;
      this.emit('ready');
    }
  }

  /**
   * Request the application to quit.
   */
  quit(): void {
    this.emit('before-quit');
    this.emit('quit');
  }

  /**
   * Convenience method: returns a promise that resolves when the app is ready.
   */
  whenReady(): Promise<void> {
    if (this.isReady) {
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      this.once('ready', resolve);
    });
  }
}

/** Singleton application instance. */
let appInstance: VoltApp | null = null;

/** Get or create the global VoltApp instance. */
export function getApp(): VoltApp {
  if (!appInstance) {
    throw new Error(
      'VoltApp not initialized. Call createApp(config) first.',
    );
  }
  return appInstance;
}

/** Create and return the global VoltApp instance. */
export function createApp(config: VoltConfig): VoltApp {
  if (appInstance) {
    throw new Error('VoltApp already initialized. Only one app instance is allowed.');
  }
  appInstance = new VoltApp(config);
  return appInstance;
}

/**
 * Reset the app instance (used for testing).
 * @internal
 */
export function resetApp(): void {
  appInstance = null;
}
