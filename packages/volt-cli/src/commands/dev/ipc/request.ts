import type { NativeIpcRequest } from '../types.js';

const IPC_PROTOTYPE_CHECK_MAX_DEPTH = 64;

export function isNativeIpcRequest(raw: unknown): raw is NativeIpcRequest {
  if (!raw || typeof raw !== 'object') {
    return false;
  }

  const value = raw as Record<string, unknown>;
  return typeof value.id === 'string' && typeof value.method === 'string';
}

export function normalizeNativeIpcRequest(raw: unknown): NativeIpcRequest | null {
  const normalizeParsed = (value: unknown): NativeIpcRequest | null => {
    if (!validatePrototypePollution(value, 0) || !isNativeIpcRequest(value)) {
      return null;
    }
    return value;
  };

  if (isNativeIpcRequest(raw)) {
    return normalizeParsed(raw);
  }

  if (typeof raw !== 'string' || raw.trim().length === 0) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as unknown;
    return normalizeParsed(parsed);
  } catch {
    return null;
  }
}

function validatePrototypePollution(value: unknown, depth: number): boolean {
  if (depth > IPC_PROTOTYPE_CHECK_MAX_DEPTH) {
    return false;
  }

  if (Array.isArray(value)) {
    return value.every((entry) => validatePrototypePollution(entry, depth + 1));
  }

  if (!value || typeof value !== 'object') {
    return true;
  }

  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (key === '__proto__' || key === 'constructor' || key === 'prototype') {
      return false;
    }
    if (!validatePrototypePollution(nested, depth + 1)) {
      return false;
    }
  }

  return true;
}
