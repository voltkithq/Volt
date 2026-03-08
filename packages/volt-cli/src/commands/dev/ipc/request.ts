import type { NativeIpcRequest } from '../types.js';

export function isNativeIpcRequest(raw: unknown): raw is NativeIpcRequest {
  if (!raw || typeof raw !== 'object') {
    return false;
  }

  const value = raw as Record<string, unknown>;
  return typeof value.id === 'string' && typeof value.method === 'string';
}

export function normalizeNativeIpcRequest(raw: unknown): NativeIpcRequest | null {
  if (isNativeIpcRequest(raw)) {
    return raw;
  }

  if (typeof raw !== 'string' || raw.trim().length === 0) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as unknown;
    return isNativeIpcRequest(parsed) ? parsed : null;
  } catch {
    return null;
  }
}
