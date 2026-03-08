import {
  NATIVE_HOST_PROTOCOL_VERSION,
  type HostToParentMessage,
} from '../native-host-protocol.js';
import { isHostToParentMessage } from './runtime-protocol.js';

export interface NativeHostMessageRouteCallbacks {
  onStarting(): void;
  onReady(): void;
  onEvent(eventJson: string): void;
  onRuntimeError(message: string): void;
  onNativeUnavailable(message: string): void;
  onPong(pingId: number): void;
  onStopping(): void;
  onStopped(): void;
  onProtocolFailure(message: string): void;
}

export function routeNativeHostMessage(
  raw: unknown,
  callbacks: NativeHostMessageRouteCallbacks,
): void {
  if (!isHostToParentMessage(raw)) {
    callbacks.onProtocolFailure('Invalid host message shape from native host');
    return;
  }

  if (raw.protocolVersion !== NATIVE_HOST_PROTOCOL_VERSION) {
    callbacks.onProtocolFailure(
      `Unsupported host protocol version: ${raw.protocolVersion} (expected ${NATIVE_HOST_PROTOCOL_VERSION})`,
    );
    return;
  }

  handleNativeHostMessage(raw, callbacks);
}

function handleNativeHostMessage(
  message: HostToParentMessage,
  callbacks: NativeHostMessageRouteCallbacks,
): void {
  switch (message.type) {
    case 'starting':
      callbacks.onStarting();
      break;
    case 'ready':
      callbacks.onReady();
      break;
    case 'event':
      callbacks.onEvent(message.eventJson);
      break;
    case 'runtime-error':
      callbacks.onRuntimeError(message.message);
      break;
    case 'native-unavailable':
      callbacks.onNativeUnavailable(message.message);
      break;
    case 'pong':
      callbacks.onPong(message.pingId);
      break;
    case 'stopping':
      callbacks.onStopping();
      break;
    case 'stopped':
      callbacks.onStopped();
      break;
    default:
      break;
  }
}
