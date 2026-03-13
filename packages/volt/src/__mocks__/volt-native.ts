/**
 * Mock implementation of @voltkit/volt-native for testing.
 * Stubs all native bindings so TypeScript tests can run without Rust compilation.
 */

import { vi } from 'vitest';

type NativeDialogFileFilter = {
  name: string;
  extensions: string[];
};

type NativeOpenDialogOptions = {
  title?: string;
  default_path?: string;
  filters?: NativeDialogFileFilter[];
  multiple?: boolean;
  directory?: boolean;
};

type NativeSaveDialogOptions = {
  title?: string;
  default_path?: string;
  filters?: NativeDialogFileFilter[];
};

type NativeMessageDialogOptions = {
  dialog_type?: 'info' | 'warning' | 'error';
  title?: string;
  message: string;
  buttons?: string[];
};

type NativeNotificationOptions = {
  title: string;
  body?: string;
  icon?: string;
};

type NativeTrayConfig = {
  tooltip?: string;
  icon?: string;
};

type NativeAppConfig = {
  name?: string;
  devtools?: boolean;
  permissions?: string[];
};

type NativeWindowCreateConfig = {
  jsId?: string;
  url?: string;
  html?: string;
  devtools?: boolean;
  transparent?: boolean;
  userAgent?: string;
  allowedOrigins?: string[];
  initScript?: string;
  window?: {
    title?: string;
    width?: number;
    height?: number;
    minWidth?: number;
    minHeight?: number;
    maxWidth?: number;
    maxHeight?: number;
    resizable?: boolean;
    decorations?: boolean;
    transparent?: boolean;
    alwaysOnTop?: boolean;
    maximized?: boolean;
    visible?: boolean;
    x?: number;
    y?: number;
  };
};

// ── Clipboard ──────────────────────────────────────────────────────────

let _clipboardText = '';

export const clipboardReadText = vi.fn(() => _clipboardText);
export const clipboardWriteText = vi.fn((text: string) => {
  _clipboardText = text;
});
export const clipboardReadImage = vi.fn(() => ({
  rgba: Buffer.alloc(16),
  width: 2,
  height: 2,
}));
export const clipboardWriteImage = vi.fn(
  (_data: { rgba: Buffer; width: number; height: number }) => {},
);

// ── Dialog ─────────────────────────────────────────────────────────────

export const dialogShowOpen = vi.fn(
  (_options: NativeOpenDialogOptions) => ['/mock/path/file.txt'] as string[],
);
export const dialogShowSave = vi.fn(
  (_options: NativeSaveDialogOptions) => '/mock/path/save.txt' as string | null,
);
export const dialogShowMessage = vi.fn(
  (_options: NativeMessageDialogOptions) => true,
);
export const dialogShowOpenWithGrant = vi.fn(
  (_options: NativeOpenDialogOptions) => ({
    paths: ['/mock/workspace'],
    grantIds: ['mock_grant_001'],
  }),
);

// ── File System ────────────────────────────────────────────────────────

export const fsReadFile = vi.fn(
  (_baseDir: string, _path: string) => Buffer.from('mock file content'),
);
export const fsReadFileText = vi.fn(
  (_baseDir: string, _path: string) => 'mock file content',
);
export const fsWriteFile = vi.fn(
  (_baseDir: string, _path: string, _data: Buffer) => {},
);
export const fsReadDir = vi.fn(
  (_baseDir: string, _path: string) => ['file1.txt', 'file2.txt'] as string[],
);
export const fsStat = vi.fn(
  (_baseDir: string, _path: string) => ({
    size: 1024,
    isFile: true,
    isDir: false,
    readonly: false,
    modifiedMs: 1700000000000,
    createdMs: 1699000000000,
  }),
);
export const fsExists = vi.fn(
  (_baseDir: string, _path: string) => true,
);
export const fsMkdir = vi.fn((_baseDir: string, _path: string) => {});
export const fsRemove = vi.fn((_baseDir: string, _path: string) => {});
export const fsResolveGrant = vi.fn(
  (_grantId: string) => '/mock/grant/path',
);

// ── Shell ──────────────────────────────────────────────────────────────

export const shellOpenExternal = vi.fn((_url: string) => {});

// ── Notification ───────────────────────────────────────────────────────

export const notificationShow = vi.fn(
  (_options: NativeNotificationOptions) => {},
);

// ── Global Shortcut ────────────────────────────────────────────────────

export class VoltGlobalShortcut {
  static instances: VoltGlobalShortcut[] = [];
  private _registered: Map<string, (accelerator: string) => void> = new Map();

  constructor() {
    VoltGlobalShortcut.instances.push(this);
  }

  register = vi.fn(
    (accelerator: string, callback: (accelerator: string) => void) => {
      this._registered.set(accelerator, callback);
    },
  );
  unregister = vi.fn((accelerator: string) => {
    this._registered.delete(accelerator);
  });
  unregisterAll = vi.fn(() => {
    this._registered.clear();
  });
  isRegistered = vi.fn((accelerator: string) => {
    return this._registered.has(accelerator);
  });
  getRegistered = vi.fn(() => {
    return Array.from(this._registered.keys());
  });

  static resetInstances = vi.fn(() => {
    VoltGlobalShortcut.instances = [];
  });
}

// ── Updater ────────────────────────────────────────────────────────────

export const updaterCheck = vi.fn(
  async (_config: { endpoint: string; public_key: string; current_version: string }) =>
    null as { version: string; url: string; signature: string; sha256: string; target: string } | null,
);
export const updaterDownloadAndVerify = vi.fn(
  async (
    _config: { endpoint: string; public_key: string; current_version: string },
    _info: { version: string; url: string; signature: string; sha256: string; target: string },
  ) => Buffer.from('mock-binary'),
);
export const updaterApply = vi.fn((_data: Buffer) => {});

// ── Tray ───────────────────────────────────────────────────────────────

export class VoltTray {
  constructor(_config: NativeTrayConfig) {}

  setTooltip = vi.fn((_tooltip: string) => {});
  setIcon = vi.fn((_iconPath: string) => {});
  setVisible = vi.fn((_visible: boolean) => {});
  onClick = vi.fn((_callback: (err: Error | null, event: string) => void) => {});
  destroy = vi.fn(() => {});
}

// ── Menu ───────────────────────────────────────────────────────────────

export class VoltMenu {
  static instances: VoltMenu[] = [];
  private _items: unknown[];

  constructor(template: unknown[]) {
    this._items = template;
    VoltMenu.instances.push(this);
  }

  setAsAppMenu = vi.fn(() => {});
  itemCount = vi.fn(() => this._items.length);

  static resetInstances = vi.fn(() => {
    VoltMenu.instances = [];
  });
}

// ── Window ──────────────────────────────────────────────────────────────

export const windowClose = vi.fn((_jsId: string) => {});
export const windowShow = vi.fn((_jsId: string) => {});
export const windowFocus = vi.fn((_jsId: string) => {});
export const windowMaximize = vi.fn((_jsId: string) => {});
export const windowMinimize = vi.fn((_jsId: string) => {});
export const windowRestore = vi.fn((_jsId: string) => {});
export const windowEvalScript = vi.fn(
  (_jsId: string, _script: string) => {},
);
export const windowCount = vi.fn(() => 0);

// ── App ────────────────────────────────────────────────────────────────

export class VoltApp {
  constructor(_config: NativeAppConfig) {}

  createWindow = vi.fn((_config: NativeWindowCreateConfig) => {});
  onEvent = vi.fn((_callback: (event: string) => void) => {});
  run = vi.fn(() => {});
}

// ── IPC ────────────────────────────────────────────────────────────────

export class VoltIpc {
  private _handlers: Map<string, (args: string) => string> = new Map();

  constructor() {}

  handle = vi.fn((channel: string, callback: (args: string) => string) => {
    this._handlers.set(channel, callback);
  });
  removeHandler = vi.fn((channel: string) => {
    this._handlers.delete(channel);
  });
  processMessage = vi.fn((raw: string) => {
    const msg = JSON.parse(raw);
    const handler = this._handlers.get(msg.method);
    if (handler) {
      const result = handler(JSON.stringify(msg.args));
      return JSON.stringify({ id: msg.id, result: JSON.parse(result) });
    }
    return JSON.stringify({ id: msg.id, error: `No handler for ${msg.method}` });
  });
  onEmit = vi.fn((_callback: (event: string) => void) => {});
  emitEvent = vi.fn((_event: string, _data: string) => {});
}
