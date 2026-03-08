import { describe, it, expect, beforeEach } from 'vitest';
import { VoltApp, createApp, getApp, resetApp } from '../app.js';
import { BrowserWindow } from '../window.js';
import type { VoltConfig } from '../types.js';

const baseConfig: VoltConfig = {
  name: 'Test App',
  version: '1.2.3',
};

describe('VoltApp', () => {
  beforeEach(() => {
    resetApp();
  });

  it('returns the app name from config', () => {
    const app = new VoltApp(baseConfig);
    expect(app.getName()).toBe('Test App');
  });

  it('returns the version from config', () => {
    const app = new VoltApp(baseConfig);
    expect(app.getVersion()).toBe('1.2.3');
  });

  it('returns default version 0.0.0 when not specified', () => {
    const app = new VoltApp({ name: 'No Version' });
    expect(app.getVersion()).toBe('0.0.0');
  });

  it('returns the full config', () => {
    const app = new VoltApp(baseConfig);
    const config = app.getConfig();
    expect(config.name).toBe('Test App');
    expect(config.version).toBe('1.2.3');
  });

  it('returns a cloned config snapshot', () => {
    const app = new VoltApp(baseConfig);
    const config = app.getConfig();
    config.name = 'Mutated';
    expect(app.getName()).toBe('Test App');
  });

  it('starts not ready', () => {
    const app = new VoltApp(baseConfig);
    expect(app.ready).toBe(false);
  });

  it('emits ready event on markReady', () => {
    const app = new VoltApp(baseConfig);
    let fired = false;
    app.on('ready', () => { fired = true; });
    app.markReady();
    expect(app.ready).toBe(true);
    expect(fired).toBe(true);
  });

  it('only fires ready event once', () => {
    const app = new VoltApp(baseConfig);
    let count = 0;
    app.on('ready', () => { count++; });
    app.markReady();
    app.markReady();
    expect(count).toBe(1);
  });

  it('whenReady resolves immediately if already ready', async () => {
    const app = new VoltApp(baseConfig);
    app.markReady();
    await expect(app.whenReady()).resolves.toBeUndefined();
  });

  it('whenReady resolves when markReady is called later', async () => {
    const app = new VoltApp(baseConfig);
    const promise = app.whenReady();
    app.markReady();
    await expect(promise).resolves.toBeUndefined();
  });

  it('quit emits before-quit then quit events in order', () => {
    const app = new VoltApp(baseConfig);
    const events: string[] = [];
    app.on('before-quit', () => events.push('before-quit'));
    app.on('quit', () => events.push('quit'));
    app.quit();
    expect(events).toEqual(['before-quit', 'quit']);
  });

  it('manages native app instance', () => {
    const app = new VoltApp(baseConfig);
    expect(app.getNativeApp()).toBeNull();
    const native = { fake: true };
    app.setNativeApp(native);
    expect(app.getNativeApp()).toBe(native);
  });
});

describe('createApp / getApp / resetApp', () => {
  beforeEach(() => {
    resetApp();
  });

  it('getApp throws before createApp', () => {
    expect(() => getApp()).toThrow('VoltApp not initialized');
  });

  it('createApp creates and returns an instance', () => {
    const app = createApp(baseConfig);
    expect(app).toBeInstanceOf(VoltApp);
    expect(app.getName()).toBe('Test App');
  });

  it('getApp returns the same instance after createApp', () => {
    const app = createApp(baseConfig);
    expect(getApp()).toBe(app);
  });

  it('createApp throws if called twice', () => {
    createApp(baseConfig);
    expect(() => createApp(baseConfig)).toThrow('already initialized');
  });

  it('resetApp allows re-creating', () => {
    createApp(baseConfig);
    resetApp();
    const app2 = createApp({ name: 'Second' });
    expect(app2.getName()).toBe('Second');
  });

  it('emits window-all-closed when the final window is destroyed', () => {
    const app = createApp(baseConfig);
    let fired = false;
    app.on('window-all-closed', () => {
      fired = true;
    });

    const first = new BrowserWindow();
    const second = new BrowserWindow();
    first.destroy();
    expect(fired).toBe(false);
    second.destroy();
    expect(fired).toBe(true);
  });
});
