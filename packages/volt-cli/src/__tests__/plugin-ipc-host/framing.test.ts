import { describe, expect, it } from 'vitest';

import {
  STREAM_METHODS,
  frameMessage,
  tryParseFrame,
  type IpcMessage,
} from '../../utils/plugin-ipc-host.js';

describe('frameMessage / tryParseFrame', () => {
  it('roundtrips a message through frame/parse', () => {
    const msg: IpcMessage = {
      type: 'request',
      id: 'abc-123',
      method: 'test.echo',
      payload: { key: 'value' },
      error: null,
    };

    const frame = frameMessage(msg);
    const parsed = tryParseFrame(frame, 0);
    expect(parsed).not.toBeNull();
    expect(parsed!.message).toEqual(msg);
    expect(parsed!.bytesConsumed).toBe(frame.length);
  });

  it('returns null for incomplete header', () => {
    expect(tryParseFrame(Buffer.alloc(2), 0)).toBeNull();
  });

  it('returns null for incomplete body', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(100, 0);
    expect(tryParseFrame(buf, 0)).toBeNull();
  });

  it('parses multiple frames from a single buffer', () => {
    const msg1: IpcMessage = { type: 'event', id: '1', method: 'a', payload: null, error: null };
    const msg2: IpcMessage = { type: 'event', id: '2', method: 'b', payload: null, error: null };
    const combined = Buffer.concat([frameMessage(msg1), frameMessage(msg2)]);

    const first = tryParseFrame(combined, 0);
    expect(first).not.toBeNull();
    expect(first!.message!.id).toBe('1');

    const second = tryParseFrame(combined, first!.bytesConsumed);
    expect(second).not.toBeNull();
    expect(second!.message!.id).toBe('2');
  });

  it('rejects zero-length frame', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(0, 0);
    expect(tryParseFrame(buf, 0)).toBeNull();
  });

  it('rejects oversized frame', () => {
    const buf = Buffer.alloc(4);
    buf.writeUInt32LE(17 * 1024 * 1024, 0);
    expect(tryParseFrame(buf, 0)).toBeNull();
  });

  it('returns null message for invalid JSON', () => {
    const badJson = Buffer.from('not valid json!\n', 'utf-8');
    const header = Buffer.alloc(4);
    header.writeUInt32LE(badJson.length, 0);
    const parsed = tryParseFrame(Buffer.concat([header, badJson]), 0);
    expect(parsed).not.toBeNull();
    expect(parsed!.message).toBeNull();
    expect(parsed!.bytesConsumed).toBe(4 + badJson.length);
  });

  it('returns null message for partial JSON', () => {
    const partialJson = Buffer.from('{"type":"req\n', 'utf-8');
    const header = Buffer.alloc(4);
    header.writeUInt32LE(partialJson.length, 0);
    const parsed = tryParseFrame(Buffer.concat([header, partialJson]), 0);
    expect(parsed).not.toBeNull();
    expect(parsed!.message).toBeNull();
    expect(parsed!.bytesConsumed).toBe(4 + partialJson.length);
  });
});

describe('STREAM_METHODS', () => {
  it('has all required stream method constants', () => {
    expect(STREAM_METHODS.START).toBe('stream:start');
    expect(STREAM_METHODS.CHUNK).toBe('stream:chunk');
    expect(STREAM_METHODS.END).toBe('stream:end');
    expect(STREAM_METHODS.ERROR).toBe('stream:error');
    expect(STREAM_METHODS.PAUSE).toBe('stream:pause');
    expect(STREAM_METHODS.RESUME).toBe('stream:resume');
  });
});
