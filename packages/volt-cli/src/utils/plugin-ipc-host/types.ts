export type MessageType = 'request' | 'response' | 'event' | 'signal';

export interface IpcMessage {
  type: MessageType;
  id: string;
  method: string;
  payload: Record<string, unknown> | null;
  error: { code: string; message: string } | null;
}

export const STREAM_METHODS = {
  START: 'stream:start',
  CHUNK: 'stream:chunk',
  END: 'stream:end',
  ERROR: 'stream:error',
  PAUSE: 'stream:pause',
  RESUME: 'stream:resume',
} as const;
