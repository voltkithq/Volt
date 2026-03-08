import { describe, it, expect, beforeEach, vi } from 'vitest';
import { pathToFileURL } from 'node:url';
vi.mock('@voltkit/volt-native', async () => {
  const mod = await import('../__mocks__/volt-native.js');
  return mod;
});
import {
  windowClose,
  windowFocus,
  windowMaximize,
  windowMinimize,
  windowRestore,
  windowShow,
} from '@voltkit/volt-native';
import { BrowserWindow } from '../window.js';

describe('BrowserWindow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Clean up all windows from previous tests
    for (const win of BrowserWindow.getAllWindows()) {
      win.destroy();
    }
  });

  it('generates a unique UUID id', () => {
    const win = new BrowserWindow();
    expect(win.getId()).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    );
  });

  it('two windows have different ids', () => {
    const a = new BrowserWindow();
    const b = new BrowserWindow();
    expect(a.getId()).not.toBe(b.getId());
  });

  it('applies default options', () => {
    const win = new BrowserWindow();
    expect(win.getSize()).toEqual([800, 600]);
    expect(win.getTitle()).toBe('Volt');
    expect(win.isResizable()).toBe(true);
  });

  it('accepts custom options', () => {
    const win = new BrowserWindow({
      width: 1024,
      height: 768,
      title: 'Custom',
      resizable: false,
    });
    expect(win.getSize()).toEqual([1024, 768]);
    expect(win.getTitle()).toBe('Custom');
    expect(win.isResizable()).toBe(false);
  });

  it('loadURL sets the URL', () => {
    const win = new BrowserWindow();
    win.loadURL('https://example.com');
    expect(win.getURL()).toBe('https://example.com/');
  });

  it('loadURL rejects unsafe protocols', () => {
    const win = new BrowserWindow();
    expect(() => win.loadURL('javascript:alert(1)')).toThrow('Unsupported URL protocol');
    expect(() => win.loadURL('data:text/html,<h1>x</h1>')).toThrow('Unsupported URL protocol');
  });

  it('loadFile sets a file:// URL', () => {
    const win = new BrowserWindow();
    const filePath = '/app/index.html';
    win.loadFile(filePath);
    expect(win.getURL()).toBe(pathToFileURL(filePath).href);
  });

  it('setTitle / getTitle work', () => {
    const win = new BrowserWindow();
    win.setTitle('New Title');
    expect(win.getTitle()).toBe('New Title');
  });

  it('setSize emits resize event', () => {
    const win = new BrowserWindow();
    let resized = false;
    win.on('resize', () => { resized = true; });
    win.setSize(1920, 1080);
    expect(win.getSize()).toEqual([1920, 1080]);
    expect(resized).toBe(true);
  });

  it('setPosition emits move event', () => {
    const win = new BrowserWindow();
    let moved = false;
    win.on('move', () => { moved = true; });
    win.setPosition(100, 200);
    expect(win.getPosition()).toEqual([100, 200]);
    expect(moved).toBe(true);
  });

  it('getPosition returns [0,0] by default', () => {
    const win = new BrowserWindow();
    expect(win.getPosition()).toEqual([0, 0]);
  });

  it('setResizable updates resizable state', () => {
    const win = new BrowserWindow({ resizable: true });
    win.setResizable(false);
    expect(win.isResizable()).toBe(false);
  });

  it('setAlwaysOnTop / isAlwaysOnTop work', () => {
    const win = new BrowserWindow();
    expect(win.isAlwaysOnTop()).toBe(false);
    win.setAlwaysOnTop(true);
    expect(win.isAlwaysOnTop()).toBe(true);
  });

  it('maximize emits maximize event', () => {
    const win = new BrowserWindow();
    let maximized = false;
    win.on('maximize', () => { maximized = true; });
    win.maximize();
    expect(maximized).toBe(true);
    expect(windowMaximize).toHaveBeenCalledWith(win.getId());
  });

  it('minimize emits minimize event', () => {
    const win = new BrowserWindow();
    let minimized = false;
    win.on('minimize', () => { minimized = true; });
    win.minimize();
    expect(minimized).toBe(true);
    expect(windowMinimize).toHaveBeenCalledWith(win.getId());
  });

  it('restore emits restore event', () => {
    const win = new BrowserWindow();
    let restored = false;
    win.on('restore', () => { restored = true; });
    win.restore();
    expect(restored).toBe(true);
    expect(windowRestore).toHaveBeenCalledWith(win.getId());
  });

  it('show dispatches native show command', () => {
    const win = new BrowserWindow();
    win.show();
    expect(windowShow).toHaveBeenCalledWith(win.getId());
  });

  it('focus emits focus event and dispatches native focus command', () => {
    const win = new BrowserWindow();
    let focused = false;
    win.on('focus', () => { focused = true; });
    win.focus();
    expect(focused).toBe(true);
    expect(windowFocus).toHaveBeenCalledWith(win.getId());
    expect(BrowserWindow.getFocusedWindow()).toBe(win);
  });

  it('close emits close then closed events in order', () => {
    const win = new BrowserWindow();
    const events: string[] = [];
    win.on('close', () => { events.push('close'); });
    win.on('closed', () => { events.push('closed'); });
    win.close();
    expect(win.isDestroyed()).toBe(true);
    expect(events).toEqual(['close', 'closed']);
    expect(windowClose).toHaveBeenCalledWith(win.getId());
  });

  it('close can be prevented by close event listeners', () => {
    const win = new BrowserWindow();
    win.on('close', (event: { preventDefault(): void }) => event.preventDefault());
    win.close();
    expect(win.isDestroyed()).toBe(false);
    expect(windowClose).not.toHaveBeenCalled();
    win.destroy();
  });

  it('destroy is idempotent (double-destroy is safe)', () => {
    const win = new BrowserWindow();
    win.destroy();
    expect(() => win.destroy()).not.toThrow();
    expect(win.isDestroyed()).toBe(true);
  });

  it('throws on operations after destroy', () => {
    const win = new BrowserWindow();
    win.destroy();
    expect(() => win.loadURL('https://x.com')).toThrow('destroyed');
    expect(() => win.setTitle('x')).toThrow('destroyed');
    expect(() => win.setSize(100, 100)).toThrow('destroyed');
    expect(() => win.setPosition(0, 0)).toThrow('destroyed');
    expect(() => win.maximize()).toThrow('destroyed');
    expect(() => win.minimize()).toThrow('destroyed');
    expect(() => win.restore()).toThrow('destroyed');
  });

  it('getNativeConfig returns correct structure', () => {
    const win = new BrowserWindow({
      width: 500,
      height: 400,
      title: 'Config Test',
      transparent: true,
    });
    win.loadURL('https://example.com');
    const config = win.getNativeConfig();
    expect(config.window).toBeDefined();
    const w = config.window as Record<string, unknown>;
    expect(w.width).toBe(500);
    expect(w.height).toBe(400);
    expect(w.title).toBe('Config Test');
    expect(w.transparent).toBe(true);
    expect(config.url).toBe('https://example.com/');
    expect(config.devtools).toBe(true);
  });
});

describe('BrowserWindow static methods', () => {
  beforeEach(() => {
    for (const win of BrowserWindow.getAllWindows()) {
      win.destroy();
    }
  });

  it('getAllWindows returns all open windows', () => {
    expect(BrowserWindow.getAllWindows()).toHaveLength(0);
    const a = new BrowserWindow();
    const b = new BrowserWindow();
    expect(BrowserWindow.getAllWindows()).toHaveLength(2);
    a.destroy();
    expect(BrowserWindow.getAllWindows()).toHaveLength(1);
    b.destroy();
    expect(BrowserWindow.getAllWindows()).toHaveLength(0);
  });

  it('getFocusedWindow updates only from focus events', () => {
    expect(BrowserWindow.getFocusedWindow()).toBeNull();
    const a = new BrowserWindow();
    expect(BrowserWindow.getFocusedWindow()).toBeNull();
    a.emit('focus');
    expect(BrowserWindow.getFocusedWindow()).toBe(a);
    const b = new BrowserWindow();
    expect(BrowserWindow.getFocusedWindow()).toBe(a);
    b.emit('focus');
    expect(BrowserWindow.getFocusedWindow()).toBe(b);
  });

  it('getFocusedWindow becomes null when focused window is destroyed', () => {
    const a = new BrowserWindow();
    const b = new BrowserWindow();
    b.emit('focus');
    b.destroy();
    expect(BrowserWindow.getFocusedWindow()).toBeNull();
    // a is still open but not focused
    a.destroy();
  });

  it('fromId retrieves window by ID', () => {
    const win = new BrowserWindow();
    expect(BrowserWindow.fromId(win.getId())).toBe(win);
    expect(BrowserWindow.fromId('nonexistent')).toBeUndefined();
  });
});
