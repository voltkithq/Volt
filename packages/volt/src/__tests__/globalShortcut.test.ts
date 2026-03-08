import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('@voltkit/volt-native', async () => {
  return import('../__mocks__/volt-native.js');
});

import { globalShortcut } from '../globalShortcut.js';
import { VoltGlobalShortcut } from '@voltkit/volt-native';

type ShortcutMockState = {
  instances: Array<{
    register: ReturnType<typeof vi.fn>;
    unregister: ReturnType<typeof vi.fn>;
  }>;
};

function shortcutState(): ShortcutMockState {
  return VoltGlobalShortcut as unknown as ShortcutMockState;
}

describe('globalShortcut module', () => {
  beforeEach(() => {
    globalShortcut.unregisterAll();
  });

  it('register returns true on first registration', () => {
    const result = globalShortcut.register('CmdOrCtrl+A', () => {});
    expect(result).toBe(true);
  });

  it('register returns false on duplicate registration', () => {
    globalShortcut.register('CmdOrCtrl+B', () => {});
    const result = globalShortcut.register('CmdOrCtrl+B', () => {});
    expect(result).toBe(false);
  });

  it('does not keep stale registration state if native register throws', () => {
    const instance = shortcutState().instances[shortcutState().instances.length - 1];
    instance.register.mockImplementationOnce(() => {
      throw new Error('native register failed');
    });

    expect(() => globalShortcut.register('CmdOrCtrl+G', () => {})).toThrow('native register failed');
    expect(globalShortcut.register('CmdOrCtrl+G', () => {})).toBe(true);
  });

  it('isRegistered returns true for registered shortcuts', () => {
    globalShortcut.register('CmdOrCtrl+C', () => {});
    expect(globalShortcut.isRegistered('CmdOrCtrl+C')).toBe(true);
  });

  it('isRegistered returns false for unregistered shortcuts', () => {
    expect(globalShortcut.isRegistered('CmdOrCtrl+Z')).toBe(false);
  });

  it('unregister removes a specific shortcut', () => {
    globalShortcut.register('CmdOrCtrl+D', () => {});
    globalShortcut.unregister('CmdOrCtrl+D');
    expect(globalShortcut.isRegistered('CmdOrCtrl+D')).toBe(false);
  });

  it('keeps callback map intact if native unregister throws', () => {
    globalShortcut.register('CmdOrCtrl+L', () => {});
    const instance = shortcutState().instances[shortcutState().instances.length - 1];
    instance.unregister.mockImplementationOnce(() => {
      throw new Error('native unregister failed');
    });

    expect(() => globalShortcut.unregister('CmdOrCtrl+L')).toThrow('native unregister failed');
    expect(globalShortcut.register('CmdOrCtrl+L', () => {})).toBe(false);
  });

  it('unregisterAll clears all shortcuts', () => {
    globalShortcut.register('CmdOrCtrl+1', () => {});
    globalShortcut.register('CmdOrCtrl+2', () => {});
    globalShortcut.unregisterAll();
    expect(globalShortcut.isRegistered('CmdOrCtrl+1')).toBe(false);
    expect(globalShortcut.isRegistered('CmdOrCtrl+2')).toBe(false);
  });

  it('registering multiple different shortcuts works', () => {
    globalShortcut.register('CmdOrCtrl+X', () => {});
    globalShortcut.register('CmdOrCtrl+Y', () => {});
    globalShortcut.register('CmdOrCtrl+Z', () => {});
    expect(globalShortcut.isRegistered('CmdOrCtrl+X')).toBe(true);
    expect(globalShortcut.isRegistered('CmdOrCtrl+Y')).toBe(true);
    expect(globalShortcut.isRegistered('CmdOrCtrl+Z')).toBe(true);
  });

  it('rejects invalid accelerator formats', () => {
    expect(() => globalShortcut.register('not-a-shortcut', () => {})).toThrow(
      'Invalid accelerator',
    );
    expect(() => globalShortcut.register('Ctrl+Shift', () => {})).toThrow(
      'Exactly one non-modifier key token is required',
    );
  });
});
