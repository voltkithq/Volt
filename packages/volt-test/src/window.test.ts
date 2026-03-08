import { describe, expect, it } from 'vitest';
import { assertWindowReady, parseWindowStatus, waitForWindowStatus } from './window.js';

describe('window helpers', () => {
  it('parses runtime window status payload', () => {
    const parsed = parseWindowStatus({
      runtime: {
        windowCount: 1,
        nativeReady: true,
        shortcut: 'CmdOrCtrl+Shift+P',
      },
    });

    expect(parsed).toEqual({
      windowCount: 1,
      nativeReady: true,
      shortcut: 'CmdOrCtrl+Shift+P',
    });
  });

  it('asserts ready window status', () => {
    expect(() =>
      assertWindowReady({
        windowCount: 1,
        nativeReady: true,
      }),
    ).not.toThrow();
  });

  it('waits for status predicate to become true', async () => {
    const payloads = [
      { runtime: { windowCount: 0, nativeReady: false } },
      { runtime: { windowCount: 1, nativeReady: true } },
    ];
    let index = 0;

    const status = await waitForWindowStatus(
      async () => {
        const payload = payloads[Math.min(index, payloads.length - 1)];
        index += 1;
        return payload;
      },
      (value) => value.windowCount >= 1 && value.nativeReady,
      {
        timeoutMs: 1_000,
        intervalMs: 1,
        description: 'native ready status',
      },
    );

    expect(status.windowCount).toBe(1);
    expect(status.nativeReady).toBe(true);
  });
});
