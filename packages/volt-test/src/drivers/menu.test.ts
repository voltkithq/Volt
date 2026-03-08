import { describe, expect, it } from 'vitest';
import { MenuAutomationDriver } from './menu.js';

describe('MenuAutomationDriver', () => {
  it('parses a valid setup payload', () => {
    const driver = new MenuAutomationDriver();
    const state = driver.parseSetupPayload({
      shortcut: 'CmdOrCtrl+Shift+P',
      shortcutRegistered: true,
    });

    expect(state.shortcut).toBe('CmdOrCtrl+Shift+P');
    expect(state.shortcutRegistered).toBe(true);
  });

  it('rejects invalid setup payloads', () => {
    const driver = new MenuAutomationDriver();
    expect(() => driver.parseSetupPayload({ shortcutRegistered: true })).toThrow('shortcut string');
    expect(() => driver.parseSetupPayload({ shortcut: 'A', shortcutRegistered: 'yes' })).toThrow(
      'shortcutRegistered boolean',
    );
  });

  it('counts menu click events by configured name', () => {
    const driver = new MenuAutomationDriver();
    const count = driver.countClickEvents([
      { event: 'demo:menu-click', payload: { menuId: 'demo:quit' } },
      { event: 'demo:shortcut', payload: {} },
      { event: 'demo:menu-click', payload: { menuId: 'demo:refresh-status' } },
    ]);

    expect(count).toBe(2);
  });
});
