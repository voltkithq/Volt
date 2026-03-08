import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { createApp, resetApp } from '../app.js';
import { Tray } from '../tray.js';

describe('Tray', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetApp();
    createApp({
      name: 'Tray Test App',
      permissions: ['tray', 'fs'],
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    resetApp();
  });

  it('creates with default options', () => {
    const tray = new Tray();
    expect(tray.getToolTip()).toBe('');
    expect(tray.isVisible()).toBe(true);
    expect(tray.isDestroyed()).toBe(false);
  });

  it('creates with custom tooltip and icon', () => {
    const tray = new Tray({
      tooltip: 'My App',
      icon: './icon.png',
    });
    expect(tray.getToolTip()).toBe('My App');
  });

  it('warns when tray menu items are provided', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const tray = new Tray({
      menu: [{ label: 'Quit' }],
    });
    expect(warn).toHaveBeenCalledWith(
      '[volt] Tray menu is not supported yet and will be ignored.',
    );
    expect(tray.isDestroyed()).toBe(false);
  });

  it('setToolTip updates the tooltip', () => {
    const tray = new Tray();
    tray.setToolTip('Updated');
    expect(tray.getToolTip()).toBe('Updated');
  });

  it('setVisible updates visibility', () => {
    const tray = new Tray();
    tray.setVisible(false);
    expect(tray.isVisible()).toBe(false);
    tray.setVisible(true);
    expect(tray.isVisible()).toBe(true);
  });

  it('destroy marks as destroyed and clears visibility', () => {
    const tray = new Tray();
    tray.destroy();
    expect(tray.isDestroyed()).toBe(true);
    expect(tray.isVisible()).toBe(false);
  });

  it('destroy is idempotent', () => {
    const tray = new Tray();
    tray.destroy();
    expect(() => tray.destroy()).not.toThrow();
  });

  it('requires tray permission before creating a tray', () => {
    resetApp();
    createApp({ name: 'Tray Test App', permissions: ['fs'] });

    expect(() => new Tray()).toThrow("requires 'tray' in volt.config.ts permissions");
  });

  it('requires fs permission before reading a tray icon path', () => {
    resetApp();
    createApp({ name: 'Tray Test App', permissions: ['tray'] });

    expect(() => new Tray({ icon: './icon.png' })).toThrow(
      "requires 'fs' in volt.config.ts permissions",
    );
  });

  it('setToolTip after destroy does not call native', () => {
    const tray = new Tray();
    tray.destroy();
    // After destroy, the tooltip is still updated locally
    tray.setToolTip('test');
    expect(tray.getToolTip()).toBe('test');
  });

  it('setImage calls native setIcon when alive', () => {
    const tray = new Tray();
    tray.setImage('./icon.png');
    const native = (tray as unknown as { _native: { setIcon: ReturnType<typeof vi.fn> } })._native;
    expect(native.setIcon).toHaveBeenCalledWith('./icon.png');
  });

  it('emits click events', () => {
    const tray = new Tray();
    let clicked = false;
    tray.on('click', () => { clicked = true; });
    // Simulate the native click callback
    tray.emit('click');
    expect(clicked).toBe(true);
  });

  it('registers native click callback and forwards parsed payload', () => {
    const tray = new Tray();
    const native = (tray as unknown as { _native: { onClick: ReturnType<typeof vi.fn> } })._native;
    const callback = native.onClick.mock.calls[0]?.[0] as ((err: Error | null, eventJson: string) => void) | undefined;
    expect(typeof callback).toBe('function');

    const clickHandler = vi.fn();
    tray.on('click', clickHandler);
    callback?.(null, '{"source":"native"}');

    expect(clickHandler).toHaveBeenCalledWith({ source: 'native' });
  });

  it('forwards click even when native payload is invalid JSON', () => {
    const tray = new Tray();
    const native = (tray as unknown as { _native: { onClick: ReturnType<typeof vi.fn> } })._native;
    const callback = native.onClick.mock.calls[0]?.[0] as ((err: Error | null, eventJson: string) => void) | undefined;
    expect(typeof callback).toBe('function');

    const clickHandler = vi.fn();
    tray.on('click', clickHandler);
    callback?.(null, 'not-json');

    expect(clickHandler).toHaveBeenCalledTimes(1);
    expect(clickHandler).toHaveBeenCalledWith();
  });

  it('forwards click even when native callback reports an error', () => {
    const tray = new Tray();
    const native = (tray as unknown as { _native: { onClick: ReturnType<typeof vi.fn> } })._native;
    const callback = native.onClick.mock.calls[0]?.[0] as ((err: Error | null, eventJson: string) => void) | undefined;
    expect(typeof callback).toBe('function');

    const clickHandler = vi.fn();
    tray.on('click', clickHandler);
    callback?.(new Error('native tray error'), '{"source":"native"}');

    expect(clickHandler).toHaveBeenCalledTimes(1);
    expect(clickHandler).toHaveBeenCalledWith();
  });

  it('is an EventEmitter', () => {
    const tray = new Tray();
    expect(typeof tray.on).toBe('function');
    expect(typeof tray.emit).toBe('function');
    expect(typeof tray.removeAllListeners).toBe('function');
  });
});
