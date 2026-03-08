import { EventEmitter } from 'node:events';
import { pathToFileURL } from 'node:url';
import { getApp } from '../app.js';
import type { WindowOptions } from '../types.js';
import { invokeNativeWindowCommand, type NativeWindowCommandName } from './native-bridge.js';
import {
  addWindowToRegistry,
  blurWindowInRegistry,
  focusWindowInRegistry,
  getFocusedRegisteredWindow,
  getRegisteredWindowById,
  getRegisteredWindows,
  removeWindowFromRegistry,
} from './registry.js';
import type {
  BrowserWindowCloseEvent,
  BrowserWindowLocalEvent,
  BrowserWindowMutableOptions,
} from './types.js';

function resolveWindowOptions(options: WindowOptions): BrowserWindowMutableOptions {
  return {
    width: 800,
    height: 600,
    title: 'Volt',
    resizable: true,
    decorations: true,
    devtools: process.env.NODE_ENV !== 'production',
    ...options,
  };
}

function createCloseEvent(): BrowserWindowCloseEvent {
  let prevented = false;
  return {
    get defaultPrevented() {
      return prevented;
    },
    preventDefault() {
      prevented = true;
    },
  };
}

export class BrowserWindow extends EventEmitter {
  private id: string;
  private options: BrowserWindowMutableOptions;
  private destroyed = false;
  private loadedUrl: string | null = null;
  private pendingCloseFallbackTimer: NodeJS.Timeout | null = null;

  constructor(options: WindowOptions = {}) {
    super();
    this.id = crypto.randomUUID();
    this.options = resolveWindowOptions(options);
    addWindowToRegistry(this);
    this.on('focus', () => {
      focusWindowInRegistry(this);
    });
    this.on('blur', () => {
      blurWindowInRegistry(this);
    });
  }

  getId(): string {
    return this.id;
  }

  loadURL(url: string): void {
    this.assertNotDestroyed();
    const parsed = new URL(url);
    const protocol = parsed.protocol.toLowerCase();
    if (protocol !== 'http:' && protocol !== 'https:' && protocol !== 'volt:') {
      throw new Error(`Unsupported URL protocol "${protocol}". Allowed protocols are http:, https:, and volt:.`);
    }
    this.loadedUrl = parsed.toString();
  }

  loadFile(filePath: string): void {
    this.assertNotDestroyed();
    this.loadedUrl = pathToFileURL(filePath).href;
  }

  getURL(): string | null {
    return this.loadedUrl;
  }

  setTitle(title: string): void {
    this.assertNotDestroyed();
    this.options.title = title;
  }

  getTitle(): string {
    return this.options.title;
  }

  setSize(width: number, height: number): void {
    this.assertNotDestroyed();
    this.options.width = width;
    this.options.height = height;
    this.emit('resize');
  }

  getSize(): [number, number] {
    return [this.options.width, this.options.height];
  }

  setPosition(x: number, y: number): void {
    this.assertNotDestroyed();
    this.options.x = x;
    this.options.y = y;
    this.emit('move');
  }

  getPosition(): [number, number] {
    return [this.options.x ?? 0, this.options.y ?? 0];
  }

  setResizable(resizable: boolean): void {
    this.assertNotDestroyed();
    this.options.resizable = resizable;
  }

  isResizable(): boolean {
    return this.options.resizable;
  }

  setAlwaysOnTop(flag: boolean): void {
    this.assertNotDestroyed();
    this.options.alwaysOnTop = flag;
  }

  isAlwaysOnTop(): boolean {
    return this.options.alwaysOnTop ?? false;
  }

  maximize(): void {
    this.dispatchWindowCommand('windowMaximize', 'maximize');
  }

  minimize(): void {
    this.dispatchWindowCommand('windowMinimize', 'minimize');
  }

  restore(): void {
    this.dispatchWindowCommand('windowRestore', 'restore');
  }

  show(): void {
    this.assertNotDestroyed();
    invokeNativeWindowCommand('windowShow', this.id);
  }

  focus(): void {
    this.dispatchWindowCommand('windowFocus', 'focus');
  }

  close(): void {
    this.assertNotDestroyed();
    const event = createCloseEvent();
    this.emit('close', event);
    if (event.defaultPrevented) {
      return;
    }

    const dispatchMode = invokeNativeWindowCommand('windowClose', this.id);
    if (dispatchMode === 'runtime') {
      this.scheduleNativeCloseFallback();
      return;
    }
    this.destroy();
  }

  destroy(): void {
    if (this.destroyed) {
      return;
    }

    this.clearNativeCloseFallback();
    this.destroyed = true;
    const noWindowsRemain = removeWindowFromRegistry(this);
    if (noWindowsRemain) {
      try {
        getApp().emit('window-all-closed');
      } catch {
        // App may not be initialized in isolated tests or utility scripts.
      }
    }

    this.emit('closed');
    this.removeAllListeners();
  }

  isDestroyed(): boolean {
    return this.destroyed;
  }

  getNativeConfig(): Record<string, unknown> {
    return {
      jsId: this.id,
      window: {
        title: this.options.title,
        width: this.options.width,
        height: this.options.height,
        minWidth: this.options.minWidth,
        minHeight: this.options.minHeight,
        maxWidth: this.options.maxWidth,
        maxHeight: this.options.maxHeight,
        resizable: this.options.resizable,
        decorations: this.options.decorations,
        transparent: this.options.transparent,
        alwaysOnTop: this.options.alwaysOnTop,
        maximized: this.options.maximized,
        x: this.options.x,
        y: this.options.y,
      },
      url: this.loadedUrl,
      devtools: this.options.devtools,
    };
  }

  private dispatchWindowCommand(
    command: NativeWindowCommandName,
    eventName?: BrowserWindowLocalEvent,
  ): void {
    this.assertNotDestroyed();
    invokeNativeWindowCommand(command, this.id);
    if (eventName) {
      this.emit(eventName);
    }
  }

  private assertNotDestroyed(): void {
    if (this.destroyed) {
      throw new Error('Window has been destroyed');
    }
  }

  private scheduleNativeCloseFallback(): void {
    this.clearNativeCloseFallback();
    this.pendingCloseFallbackTimer = setTimeout(() => {
      this.pendingCloseFallbackTimer = null;
      if (!this.destroyed) {
        this.destroy();
      }
    }, 1000);
    if (typeof this.pendingCloseFallbackTimer.unref === 'function') {
      this.pendingCloseFallbackTimer.unref();
    }
  }

  private clearNativeCloseFallback(): void {
    if (!this.pendingCloseFallbackTimer) {
      return;
    }
    clearTimeout(this.pendingCloseFallbackTimer);
    this.pendingCloseFallbackTimer = null;
  }

  static getAllWindows(): BrowserWindow[] {
    return getRegisteredWindows<BrowserWindow>();
  }

  static getFocusedWindow(): BrowserWindow | null {
    return getFocusedRegisteredWindow<BrowserWindow>();
  }

  static fromId(id: string): BrowserWindow | undefined {
    return getRegisteredWindowById<BrowserWindow>(id);
  }
}
