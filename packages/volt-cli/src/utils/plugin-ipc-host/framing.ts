import type { IpcMessage } from './types.js';

const MAX_FRAME_SIZE = 16 * 1024 * 1024;

export interface ParsedFrame {
  message: IpcMessage | null;
  bytesConsumed: number;
}

export function frameMessage(msg: IpcMessage): Buffer {
  const body = Buffer.from(`${JSON.stringify(msg)}\n`, 'utf-8');
  if (body.length > MAX_FRAME_SIZE) {
    throw new Error(`frame too large: ${body.length} bytes exceeds ${MAX_FRAME_SIZE} byte limit`);
  }

  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.length, 0);
  return Buffer.concat([header, body]);
}

export function tryParseFrame(buffer: Buffer, offset: number): ParsedFrame | null {
  if (buffer.length - offset < 4) return null;
  const length = buffer.readUInt32LE(offset);
  if (length === 0 || length > MAX_FRAME_SIZE) return null;
  if (buffer.length - offset - 4 < length) return null;

  const jsonBytes = buffer.subarray(offset + 4, offset + 4 + length);
  const raw = jsonBytes.toString('utf-8');
  const stripped = raw.endsWith('\n') ? raw.slice(0, -1) : raw;

  try {
    return {
      message: JSON.parse(stripped) as IpcMessage,
      bytesConsumed: 4 + length,
    };
  } catch {
    return {
      message: null,
      bytesConsumed: 4 + length,
    };
  }
}
