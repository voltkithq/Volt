import type { HostToParentMessage } from '../native-host-protocol.js';

export const DEFAULT_NATIVE_HOST_STARTUP_TIMEOUT_MS = 5000;
export const DEFAULT_NATIVE_HOST_HEARTBEAT_INTERVAL_MS = 2000;
export const DEFAULT_NATIVE_HOST_HEARTBEAT_TIMEOUT_MS = 4000;

export function isHostToParentMessage(raw: unknown): raw is HostToParentMessage {
  if (!raw || typeof raw !== 'object') {
    return false;
  }

  const value = raw as Record<string, unknown>;
  if (typeof value.type !== 'string') {
    return false;
  }
  if (typeof value.protocolVersion !== 'number') {
    return false;
  }

  switch (value.type) {
    case 'starting':
    case 'ready':
    case 'stopping':
      return true;
    case 'event':
      return typeof value.eventJson === 'string';
    case 'runtime-error':
    case 'native-unavailable':
      return typeof value.message === 'string';
    case 'pong':
      return typeof value.pingId === 'number';
    case 'stopped':
      return value.code === undefined || typeof value.code === 'number';
    default:
      return false;
  }
}
