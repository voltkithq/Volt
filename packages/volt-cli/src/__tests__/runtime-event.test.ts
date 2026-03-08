import { describe, expect, it } from 'vitest';
import { extractNativeEventJson } from '../commands/dev/runtime-event.js';

describe('native event callback argument decoding', () => {
  it('supports direct single-argument string callbacks', () => {
    expect(extractNativeEventJson(['{"type":"quit"}'])).toBe('{"type":"quit"}');
  });

  it('supports N-API err-first callbacks', () => {
    expect(extractNativeEventJson([null, '{"type":"ipc-message"}'])).toBe('{"type":"ipc-message"}');
    expect(extractNativeEventJson([undefined, '{"type":"ipc-message"}'])).toBe('{"type":"ipc-message"}');
  });

  it('returns null for unsupported callback argument shapes', () => {
    expect(extractNativeEventJson([null, null])).toBeNull();
    expect(extractNativeEventJson([new Error('boom'), '{"type":"ipc-message"}'])).toBeNull();
    expect(extractNativeEventJson([{ type: 'ipc-message' }])).toBeNull();
    expect(extractNativeEventJson([])).toBeNull();
  });
});
