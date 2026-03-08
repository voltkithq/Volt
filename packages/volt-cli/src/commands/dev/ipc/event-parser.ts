import type { NativeRuntimeEvent } from '../types.js';

function resolveIpcEventWindowId(value: Record<string, unknown>): string | null {
  if (typeof value.windowId === 'string') {
    return value.windowId;
  }
  if (typeof value.window_id === 'string') {
    return value.window_id;
  }
  if (typeof value.jsWindowId === 'string') {
    return value.jsWindowId;
  }
  if (typeof value.js_window_id === 'string') {
    return value.js_window_id;
  }
  return null;
}

function resolveIpcEventRawPayload(value: Record<string, unknown>): unknown {
  if ('raw' in value) {
    return value.raw;
  }
  if ('payload' in value) {
    return value.payload;
  }
  if ('message' in value) {
    return value.message;
  }
  if ('data' in value) {
    return value.data;
  }
  return null;
}

function resolveStringField(value: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const candidate = value[key];
    if (typeof candidate === 'string' && candidate.length > 0) {
      return candidate;
    }
  }
  return null;
}

function resolveNumericField(value: Record<string, unknown>, keys: string[]): number | null {
  for (const key of keys) {
    const candidate = value[key];
    if (typeof candidate === 'number' && Number.isFinite(candidate)) {
      return candidate;
    }
  }
  return null;
}

function normalizeNativeEventType(value: unknown): NativeRuntimeEvent['type'] | null {
  if (typeof value !== 'string') {
    return null;
  }
  const normalized = value.trim().toLowerCase().replace(/_/g, '-');
  switch (normalized) {
    case 'quit':
      return 'quit';
    case 'ipc-message':
      return 'ipc-message';
    case 'menu-event':
      return 'menu-event';
    case 'shortcut-triggered':
      return 'shortcut-triggered';
    case 'window-closed':
      return 'window-closed';
    default:
      return null;
  }
}

export function parseNativeEvent(raw: unknown): NativeRuntimeEvent | null {
  if (!raw || typeof raw !== 'object') {
    return null;
  }

  const value = raw as Record<string, unknown>;
  const eventType = normalizeNativeEventType(value.type);
  if (!eventType) {
    return null;
  }

  switch (eventType) {
    case 'quit':
      return { type: 'quit' };
    case 'ipc-message': {
      const windowId = resolveIpcEventWindowId(value);
      if (!windowId) {
        return null;
      }
      const payload = resolveIpcEventRawPayload(value);
      return {
        type: 'ipc-message',
        windowId,
        raw: payload,
      };
    }
    case 'menu-event': {
      const menuId = resolveStringField(value, ['menuId', 'menu_id']);
      if (!menuId) {
        return null;
      }
      return {
        type: 'menu-event',
        menuId,
      };
    }
    case 'shortcut-triggered': {
      const id = resolveNumericField(value, ['id', 'shortcutId', 'shortcut_id']);
      if (id === null) {
        return null;
      }
      return {
        type: 'shortcut-triggered',
        id,
      };
    }
    case 'window-closed': {
      const windowId = resolveStringField(value, ['windowId', 'window_id']);
      if (!windowId) {
        return null;
      }
      const jsWindowIdRaw = value.jsWindowId ?? value.js_window_id ?? null;
      if (jsWindowIdRaw !== null && typeof jsWindowIdRaw !== 'string') {
        return null;
      }
      return {
        type: 'window-closed',
        windowId,
        jsWindowId: jsWindowIdRaw as string | null,
      };
    }
    default:
      return null;
  }
}
