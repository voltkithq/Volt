import {
  DEFAULT_IPC_MAX_IN_FLIGHT_PER_WINDOW,
  DEFAULT_IPC_MAX_PAYLOAD_BYTES,
} from './constants.js';

export interface IpcResponse {
  id: string;
  result?: unknown;
  error?: string;
  errorCode?: string;
  errorDetails?: unknown;
}

export function createIpcResponseScript(response: IpcResponse): string {
  const responseJson = JSON.stringify(response).replace(/<\//g, '<\\/');
  return `window.__volt_response__(${JSON.stringify(responseJson)});`;
}

export function normalizeIpcPayloadBytes(value?: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) {
    return DEFAULT_IPC_MAX_PAYLOAD_BYTES;
  }
  return Math.max(1024, Math.floor(value));
}

export function normalizeIpcInFlightLimit(value?: number): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) {
    return DEFAULT_IPC_MAX_IN_FLIGHT_PER_WINDOW;
  }
  return Math.max(1, Math.floor(value));
}

export function measurePayloadBytes(value: unknown): number {
  try {
    const json = JSON.stringify(value);
    return Buffer.byteLength(json ?? 'null', 'utf8');
  } catch {
    return Number.MAX_SAFE_INTEGER;
  }
}

export function extractRequestId(raw: unknown): string {
  if (raw && typeof raw === 'object') {
    const maybeId = (raw as Record<string, unknown>).id;
    if (typeof maybeId === 'string' && maybeId.length > 0) {
      return maybeId;
    }
  }
  return 'unknown';
}
