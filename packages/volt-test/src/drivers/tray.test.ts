import { describe, expect, it } from 'vitest';
import { TrayAutomationDriver } from './tray.js';

describe('TrayAutomationDriver', () => {
  it('parses a valid setup payload', () => {
    const driver = new TrayAutomationDriver();
    const state = driver.parseSetupPayload({ trayReady: false });
    expect(state.trayReady).toBe(false);
  });

  it('rejects invalid setup payloads', () => {
    const driver = new TrayAutomationDriver();
    expect(() => driver.parseSetupPayload({})).toThrow('trayReady boolean');
    expect(() => driver.parseSetupPayload({ trayReady: 'yes' })).toThrow('trayReady boolean');
  });

  it('counts tray click events by configured name', () => {
    const driver = new TrayAutomationDriver();
    const count = driver.countClickEvents([
      { event: 'demo:tray-click', payload: {} },
      { event: 'demo:menu-click', payload: {} },
      { event: 'demo:tray-click', payload: {} },
    ]);
    expect(count).toBe(2);
  });
});
