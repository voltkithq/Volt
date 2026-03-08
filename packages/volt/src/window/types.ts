import type { WindowOptions } from '../types.js';

export type BrowserWindowMutableOptions = Required<
  Pick<WindowOptions, 'width' | 'height' | 'title' | 'resizable' | 'decorations'>
> & WindowOptions;

export type BrowserWindowLocalEvent = 'focus' | 'maximize' | 'minimize' | 'restore';

export interface BrowserWindowEvents {
  close: [event: BrowserWindowCloseEvent];
  closed: [];
  focus: [];
  blur: [];
  maximize: [];
  minimize: [];
  restore: [];
  resize: [];
  move: [];
}

export interface BrowserWindowCloseEvent {
  readonly defaultPrevented: boolean;
  preventDefault(): void;
}

export interface BrowserWindowRegistryEntry {
  getId(): string;
  isDestroyed(): boolean;
}
