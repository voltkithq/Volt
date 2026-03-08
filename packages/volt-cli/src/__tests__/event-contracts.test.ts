import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { __testOnly } from '../commands/dev.js';

function loadNativeEventFixtures(): Record<string, unknown> {
  const fixtureUrl = new URL('../../../../contracts/native-event-payloads.json', import.meta.url);
  const fixturePath = fileURLToPath(fixtureUrl);
  return JSON.parse(readFileSync(fixturePath, 'utf8')) as Record<string, unknown>;
}

describe('native event payload contracts', () => {
  it('accepts all documented contract fixtures', () => {
    const fixtures = loadNativeEventFixtures();
    for (const [name, fixture] of Object.entries(fixtures)) {
      const parsed = __testOnly.parseNativeEvent(fixture);
      expect(parsed, `fixture '${name}' should parse`).not.toBeNull();
    }
  });

  it('rejects malformed payloads', () => {
    expect(__testOnly.parseNativeEvent(null)).toBeNull();
    expect(__testOnly.parseNativeEvent({})).toBeNull();
    expect(__testOnly.parseNativeEvent({ type: 'ipc-message', windowId: 42 })).toBeNull();
    expect(__testOnly.parseNativeEvent({ type: 'menu-event', menuId: 3 })).toBeNull();
    expect(__testOnly.parseNativeEvent({ type: 'shortcut-triggered', id: '1' })).toBeNull();
    expect(__testOnly.parseNativeEvent({ type: 'window-closed', windowId: 'x', jsWindowId: 7 })).toBeNull();
    expect(__testOnly.parseNativeEvent({ type: 'unknown' })).toBeNull();
  });

  it('parses host-to-parent bridge messages', () => {
    expect(__testOnly.isHostToParentMessage({ type: 'starting', protocolVersion: 1 })).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'ready', protocolVersion: 1 })).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'event', protocolVersion: 1, eventJson: '{}' })).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'runtime-error', protocolVersion: 1, message: 'boom' })).toBe(
      true,
    );
    expect(
      __testOnly.isHostToParentMessage({ type: 'native-unavailable', protocolVersion: 1, message: 'missing' }),
    ).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'pong', protocolVersion: 1, pingId: 1 })).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'stopping', protocolVersion: 1 })).toBe(true);
    expect(__testOnly.isHostToParentMessage({ type: 'stopped', protocolVersion: 1, code: 0 })).toBe(true);

    expect(__testOnly.isHostToParentMessage({ type: 'event' })).toBe(false);
    expect(__testOnly.isHostToParentMessage({ type: 'runtime-error', protocolVersion: 1 })).toBe(false);
    expect(__testOnly.isHostToParentMessage({ type: 'pong', protocolVersion: 1, pingId: '1' })).toBe(false);
    expect(__testOnly.isHostToParentMessage({ type: 'stopped', protocolVersion: 1, code: '0' })).toBe(false);
    expect(__testOnly.isHostToParentMessage({ type: 'ready', protocolVersion: '1' })).toBe(false);
    expect(__testOnly.isHostToParentMessage({ type: 'unknown' })).toBe(false);
  });
});
