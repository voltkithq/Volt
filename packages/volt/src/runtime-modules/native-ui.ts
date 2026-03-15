declare module 'volt:menu' {
  export function setAppMenu(template: unknown): Promise<void>;
  export function on(eventName: 'click', handler: (payload: unknown) => void): void;
  export function off(eventName: 'click', handler: (payload: unknown) => void): void;
}

declare module 'volt:globalShortcut' {
  export function register(accelerator: string): Promise<number>;
  export function unregister(accelerator: string): Promise<void>;
  export function unregisterAll(): Promise<void>;
  export function on(eventName: 'triggered', handler: (payload: unknown) => void): void;
  export function off(eventName: 'triggered', handler: (payload: unknown) => void): void;
}

declare module 'volt:tray' {
  export interface TrayCreateOptions {
    tooltip?: string;
    icon?: string;
  }

  export function create(options?: TrayCreateOptions): Promise<void>;
  export function setTooltip(tooltip: string): void;
  export function setVisible(visible: boolean): void;
  export function destroy(): void;
  export function on(eventName: 'click', handler: (payload: unknown) => void): void;
  export function off(eventName: 'click', handler: (payload: unknown) => void): void;
}

declare module 'volt:notification' {
  export interface NotificationOptions {
    title: string;
    body?: string;
    icon?: string;
  }

  export function show(options: NotificationOptions): void;
}

declare module 'volt:dialog' {
  export interface FileFilter {
    name: string;
    extensions: string[];
  }

  export interface OpenDialogOptions {
    title?: string;
    defaultPath?: string;
    filters?: FileFilter[];
    multiple?: boolean;
    directory?: boolean;
  }

  export interface SaveDialogOptions {
    title?: string;
    defaultPath?: string;
    filters?: FileFilter[];
  }

  export interface MessageDialogOptions {
    dialogType?: 'info' | 'warning' | 'error';
    title?: string;
    message: string;
    buttons?: string[];
  }

  export interface GrantDialogResult {
    paths: string[];
    grantIds: string[];
  }

  export function showOpen(options?: OpenDialogOptions): Promise<string | null>;
  export function showSave(options?: SaveDialogOptions): Promise<string | null>;
  export function showMessage(options: MessageDialogOptions): Promise<0 | 1>;
  export function showOpenWithGrant(options?: OpenDialogOptions): Promise<GrantDialogResult>;
}
